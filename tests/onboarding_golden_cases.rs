//! Golden-case tests for company onboarding.
//!
//! Proves the one hand-authored behavior this module owns:
//!   onboard() creates a Company AND its head-office Branch in a single transaction,
//!   validates NPWP, and rejects duplicate company codes (including concurrently).
//! Requires DATABASE_URL (defaults to local dev Postgres on :5433).

use sqlx::{PgPool, Row};
use uuid::Uuid;

use backbone_organization::application::service::{
    validate_npwp, OnboardError, OnboardRequest, OnboardingService,
};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_organization".to_string()
    });
    PgPool::connect(&url).await.unwrap()
}

/// Unique code per test run so cases don't collide on the shared DB.
fn unique_code(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

/// A syntactically valid, run-unique 15-digit NPWP (the `npwp` column is uniquely indexed,
/// so a fixed value would collide across repeated test runs on the shared DB).
fn unique_npwp() -> String {
    let hex = Uuid::new_v4().simple().to_string();
    let digits: String = hex.chars().filter(|c| c.is_ascii_digit()).take(15).collect();
    // Pad in the unlikely event fewer than 15 digits appear in the hex.
    format!("{digits:0<15}")
}

// ── Golden case 1: onboarding creates company + head-office branch atomically ──
#[tokio::test]
async fn onboard_creates_company_and_head_office_branch() {
    let pool = pool().await;
    let svc = OnboardingService::new(pool.clone());

    let code = unique_code("ACME");
    let mut req = OnboardRequest::new(&code, "PT Acme Indonesia");
    req.npwp = Some(unique_npwp()); // run-unique 15-digit NPWP
    req.entity_type = Some("pt".to_string());

    let result = svc.onboard(req).await.expect("onboard should succeed");

    // Company row exists, active, IDR default.
    let company = sqlx::query(
        "SELECT code, legal_name, base_currency, entity_type::text AS et, status::text AS st \
         FROM organization.companies WHERE id = $1",
    )
    .bind(result.company_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(company.get::<String, _>("code"), code);
    assert_eq!(company.get::<String, _>("legal_name"), "PT Acme Indonesia");
    assert_eq!(company.get::<String, _>("base_currency"), "IDR");
    assert_eq!(company.get::<String, _>("et"), "pt");
    assert_eq!(company.get::<String, _>("st"), "active");

    // Exactly one branch, and it is the head office.
    let branches = sqlx::query(
        "SELECT id, is_head_office, branch_type::text AS bt, code, name \
         FROM organization.branches WHERE company_id = $1",
    )
    .bind(result.company_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(branches.len(), 1, "expected exactly one HQ branch");
    assert!(branches[0].get::<bool, _>("is_head_office"));
    assert_eq!(branches[0].get::<String, _>("bt"), "head_office");
    assert_eq!(branches[0].get::<String, _>("code"), "HQ");
    assert_eq!(
        branches[0].get::<Uuid, _>("id"),
        result.hq_branch_id,
        "returned hq_branch_id must match the created branch"
    );
}

// ── Golden case 2: invalid NPWP is rejected before any write ──
#[tokio::test]
async fn onboard_rejects_invalid_npwp_and_writes_nothing() {
    let pool = pool().await;
    let svc = OnboardingService::new(pool.clone());

    let code = unique_code("BADNPWP");
    let mut req = OnboardRequest::new(&code, "PT Bad Npwp");
    req.npwp = Some("12345".to_string()); // too short

    let err = svc.onboard(req).await.unwrap_err();
    assert!(matches!(err, OnboardError::InvalidNpwp(_)));
    assert_eq!(err.http_status(), 422);

    // No company created.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization.companies WHERE code = $1")
        .bind(&code)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

// ── Golden case 3: duplicate company code is rejected ──
#[tokio::test]
async fn onboard_rejects_duplicate_code() {
    let pool = pool().await;
    let svc = OnboardingService::new(pool.clone());

    let code = unique_code("DUP");
    svc.onboard(OnboardRequest::new(&code, "PT First"))
        .await
        .expect("first onboard succeeds");

    let err = svc
        .onboard(OnboardRequest::new(&code, "PT Second"))
        .await
        .unwrap_err();
    assert!(matches!(err, OnboardError::DuplicateCode(_)));
}

// ── Golden case 3b: a second company reusing an NPWP is rejected as duplicate_npwp ──
#[tokio::test]
async fn onboard_rejects_duplicate_npwp() {
    let pool = pool().await;
    let svc = OnboardingService::new(pool.clone());

    let npwp = unique_npwp();
    let mut first = OnboardRequest::new(&unique_code("NPWPA"), "PT Npwp First");
    first.npwp = Some(npwp.clone());
    svc.onboard(first).await.expect("first onboard succeeds");

    let mut second = OnboardRequest::new(&unique_code("NPWPB"), "PT Npwp Second");
    second.npwp = Some(npwp); // same NPWP, different code
    let err = svc.onboard(second).await.unwrap_err();
    assert!(
        matches!(err, OnboardError::DuplicateNpwp(_)),
        "expected DuplicateNpwp, got {err:?}"
    );
    assert_eq!(err.code(), "duplicate_npwp");
}

// ── Golden case 4: concurrent onboard of the same code yields exactly one company ──
#[tokio::test]
async fn concurrent_onboard_same_code_creates_one_company() {
    let pool = pool().await;
    let svc = OnboardingService::new(pool.clone());
    let code = unique_code("RACE");

    let (a, b) = tokio::join!(
        svc.onboard(OnboardRequest::new(&code, "PT Race A")),
        svc.onboard(OnboardRequest::new(&code, "PT Race B")),
    );

    let ok = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
    assert_eq!(ok, 1, "exactly one concurrent onboard should win");

    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization.companies WHERE code = $1")
        .bind(&code)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "DB must hold exactly one company for the code");
}

// ── Unit: NPWP validator accepts 15/16 digits, rejects others ──
#[test]
fn npwp_validator_accepts_15_and_16_digits() {
    assert!(validate_npwp("01.234.567.8-901.000")); // 15
    assert!(validate_npwp("0123456789012345")); // 16
    assert!(!validate_npwp("12345")); // too short
    assert!(!validate_npwp("012345678901234567")); // too long
    assert!(!validate_npwp("")); // empty
}
