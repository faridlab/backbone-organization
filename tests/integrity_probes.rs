//! Council integrity probes — regression tests for the CRUD-bypass hole and the tenancy hole.
//!
//! The guarded composition (`create_guarded_organization_routes`) must enforce the org
//! invariants on EVERY write path, not just onboarding:
//!   - Company has no generic write route at all (writer = onboarding only).
//!   - Branch/Department writes validate NPWP format, company existence, and same-company links.
//!   - Branch/Department writes derive their tenant from a signed token, never from the body.
//! These hit the ROUTES (via tower oneshot), not the services — closing the structural blind spot
//! the golden suite had (it only ever constructed services directly).
//! Requires DATABASE_URL (defaults to local dev Postgres on :5433).
//!
//! IGC-1..IGC-4  the CRUD-bypass and validated-write invariants.
//! IGT-1..IGT-3  the tenancy invariants (mirrors the TG-* cases backbone-pos proved).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use backbone_auth::company::CompanyVerifier;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use backbone_organization::{create_guarded_organization_routes, OrganizationModule};

const SECRET: &[u8] = b"organization-integrity-probe-secret";

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    company_id: Option<Uuid>,
}

/// Mint an HS256 access token. `company_id = None` models a token that authenticates a user but
/// carries no tenant — it must not be allowed to write.
fn token(company_id: Option<Uuid>) -> String {
    let claims = TestClaims { sub: "probe-user".into(), exp: 9_999_999_999, company_id };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(SECRET)).unwrap()
}

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_organization".to_string()
    });
    PgPool::connect(&url).await.unwrap()
}

async fn module(pool: &PgPool) -> OrganizationModule {
    OrganizationModule::builder().with_database(pool.clone()).build().unwrap()
}

fn app(m: &OrganizationModule) -> axum::Router {
    create_guarded_organization_routes(m, CompanyVerifier::hs256(SECRET))
}

/// Send a request with an optional bearer token.
async fn send_with(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: &str,
    bearer: Option<String>,
) -> (StatusCode, String) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(t) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = app.oneshot(builder.body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// Unauthenticated request.
async fn send(app: axum::Router, method: &str, uri: &str, body: &str) -> StatusCode {
    send_with(app, method, uri, body, None).await.0
}

/// Request authenticated as a principal of `company`.
async fn send_as(app: axum::Router, company: Uuid, method: &str, uri: &str, body: &str) -> StatusCode {
    send_with(app, method, uri, body, Some(token(Some(company)))).await.0
}

fn code(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

async fn seed_company(pool: &PgPool, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO organization.companies (id, code, legal_name) VALUES ($1,$2,'PT Seed')")
        .bind(id)
        .bind(code)
        .execute(pool)
        .await
        .unwrap();
    id
}

// ── IGC-1: the guarded surface mounts NO generic write route for companies ──
// Also pins the `route_layer` (not `layer`) choice: an unmatched path must 404/405, not 401.
#[tokio::test]
async fn guarded_routes_lock_company_writes() {
    let pool = pool().await;
    let body = format!(
        r#"{{"code":"{}","legalName":"PT Bypass","entityType":"pt","baseCurrency":"IDR","fiscalYearStartMonth":1,"country":"ID","isDefault":false,"status":"active"}}"#,
        code("LOCK")
    );
    // POST /companies is not routed in the guarded composition → 405/404, never 201.
    let status = send(app(&module(&pool).await), "POST", "/companies", &body).await;
    assert!(
        status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
        "guarded routes must not expose generic company create; got {status}"
    );
}

// ── IGC-2: validated branch create rejects a malformed NPWP ──
#[tokio::test]
async fn guarded_branch_rejects_bad_npwp() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("BRC")).await;
    let body = format!(r#"{{"code":"{}","name":"Cabang","npwp":"12345"}}"#, code("BR"));
    let status = send_as(app(&module(&pool).await), cid, "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "bad NPWP must be rejected");
}

// ── IGC-3: validated department create rejects a cross-company parent ──
#[tokio::test]
async fn guarded_department_rejects_cross_company_parent() {
    let pool = pool().await;
    let host = seed_company(&pool, &code("HOST")).await;
    let other = seed_company(&pool, &code("OTHER")).await;
    let foreign_parent = Uuid::new_v4();
    sqlx::query("INSERT INTO organization.departments (id, company_id, code, name) VALUES ($1,$2,'ROOT','Root')")
        .bind(foreign_parent)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();

    // The caller is a principal of `host`; the parent belongs to `other`.
    let body = format!(
        r#"{{"code":"{}","name":"Cross","parentId":"{foreign_parent}"}}"#,
        code("DEP")
    );
    let status = send_as(app(&module(&pool).await), host, "POST", "/departments", &body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "cross-company parent must be rejected"
    );
}

// ── IGC-4: the happy paths still work through the guarded surface ──
#[tokio::test]
async fn guarded_valid_writes_succeed() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("OKC")).await;

    // Valid branch (valid 15-digit NPWP).
    let branch_body = format!(r#"{{"code":"{}","name":"HQ","npwp":"012345678901234"}}"#, code("OKB"));
    let bs = send_as(app(&module(&pool).await), cid, "POST", "/branches", &branch_body).await;
    assert_eq!(bs, StatusCode::CREATED, "valid branch should be created");

    // Valid department (same-company parent).
    let parent = Uuid::new_v4();
    sqlx::query("INSERT INTO organization.departments (id, company_id, code, name) VALUES ($1,$2,$3,'Root')")
        .bind(parent)
        .bind(cid)
        .bind(code("PR"))
        .execute(&pool)
        .await
        .unwrap();
    let dep_body = format!(r#"{{"code":"{}","name":"Child","parentId":"{parent}"}}"#, code("OKD"));
    let ds = send_as(app(&module(&pool).await), cid, "POST", "/departments", &dep_body).await;
    assert_eq!(ds, StatusCode::CREATED, "valid department should be created");
}

// ── IGT-1: an unauthenticated write is rejected. Before the tenant guard this create succeeded and
// stamped whatever `companyId` the caller put in the body. ──
#[tokio::test]
async fn guarded_write_rejects_unauthenticated() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("UNAUTH")).await;
    let body = format!(r#"{{"companyId":"{cid}","code":"{}","name":"Cabang"}}"#, code("BR"));
    let status = send(app(&module(&pool).await), "POST", "/branches", &body).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "an unauthenticated write must not reach the service"
    );

    let dep_body = format!(r#"{{"companyId":"{cid}","code":"{}","name":"Dept"}}"#, code("DEP"));
    let dstatus = send(app(&module(&pool).await), "POST", "/departments", &dep_body).await;
    assert_eq!(dstatus, StatusCode::UNAUTHORIZED, "an unauthenticated dept write must not reach the service");
}

// ── IGT-2: a token that authenticates a user but carries no `company_id` claim is rejected — a
// writer that cannot name its tenant must never run. ──
#[tokio::test]
async fn guarded_write_rejects_token_without_company_id() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("NOCID")).await;
    let body = format!(r#"{{"companyId":"{cid}","code":"{}","name":"Cabang"}}"#, code("BR"));
    let (status, _) = send_with(
        app(&module(&pool).await),
        "POST",
        "/branches",
        &body,
        Some(token(None)),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a token with no tenant must not write");
}

// ── IGT-3: a `companyId` smuggled in the body is ignored — the persisted tenant is the token's.
// This is the regression that motivated the change: the body must not name the tenant. ──
#[tokio::test]
async fn body_company_id_cannot_override_the_token_tenant() {
    let pool = pool().await;
    let token_company = seed_company(&pool, &code("TOKCO")).await;
    let attacker_company = seed_company(&pool, &code("ATKCO")).await;

    // Branch: the body names the attacker's company; the token names ours.
    let branch_code = code("SMUG");
    let body = format!(
        r#"{{"companyId":"{attacker_company}","code":"{branch_code}","name":"Smuggled"}}"#
    );
    let status = send_as(app(&module(&pool).await), token_company, "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::CREATED);

    let persisted: Uuid =
        sqlx::query_scalar("SELECT company_id FROM organization.branches WHERE code = $1")
            .bind(&branch_code)
            .fetch_one(&pool)
            .await
            .expect("branch row");
    assert_eq!(persisted, token_company, "tenant must come from the token, not the body");
    assert_ne!(persisted, attacker_company, "the body's companyId must be ignored");

    // Department: same smuggle, same verdict.
    let dept_code = code("SMUGD");
    let dbody = format!(
        r#"{{"companyId":"{attacker_company}","code":"{dept_code}","name":"Smuggled"}}"#
    );
    let dstatus =
        send_as(app(&module(&pool).await), token_company, "POST", "/departments", &dbody).await;
    assert_eq!(dstatus, StatusCode::CREATED);

    let dpersisted: Uuid =
        sqlx::query_scalar("SELECT company_id FROM organization.departments WHERE code = $1")
            .bind(&dept_code)
            .fetch_one(&pool)
            .await
            .expect("department row");
    assert_eq!(dpersisted, token_company, "tenant must come from the token, not the body");
    assert_ne!(dpersisted, attacker_company, "the body's companyId must be ignored");
}

// IGC-5: onboarding must pass the RLS fence as a NON-OWNER role. The head-office branch table is
// FORCE row-level-security fenced (company subtree of the session's bound company) and the onboard
// flow runs before any tenant exists — every other probe runs on the owner DSN, and the table
// owner BYPASSES row-level security, so an unscoped branch insert passed them all while failing
// (500) on any app-style role. The service binds the freshly minted company onto the transaction;
// this probe fails if that binding is ever lost. Requires the fence migrations to be applied and
// DATABASE_URL to be a role that may CREATE ROLE (the dev owner DSN qualifies).
#[tokio::test]
async fn onboard_passes_rls_as_non_owner_role() {
    const ROLE: &str = "org_rls_probe";
    const PW: &str = "org_rls_probe_pw";
    let admin = pool().await;

    // Precondition: the probe only means something when the branch fence is live.
    let fenced: bool = sqlx::query_scalar(
        "SELECT relforcerowsecurity FROM pg_class WHERE oid = 'organization.branches'::regclass",
    )
    .fetch_one(&admin)
    .await
    .unwrap();
    assert!(
        fenced,
        "organization.branches must have FORCE RLS for this probe to mean anything"
    );

    sqlx::raw_sql(&format!(
        "DO $$ BEGIN \
           IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{ROLE}') THEN \
             EXECUTE 'DROP OWNED BY {ROLE}'; \
           END IF; \
         END $$; \
         DROP ROLE IF EXISTS {ROLE}; \
         CREATE ROLE {ROLE} LOGIN PASSWORD '{PW}'; \
         GRANT USAGE ON SCHEMA organization TO {ROLE}; \
         GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA organization TO {ROLE};"
    ))
    .execute(&admin)
    .await
    .expect("probe role bootstrap needs an owner-capable DATABASE_URL");

    let after_at = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/backbone_organization".to_string());
    let after_at = after_at.rsplit('@').next().unwrap();
    let app_pool = PgPool::connect(&format!("postgresql://{ROLE}:{PW}@{after_at}")).await.unwrap();

    // Onboarding is unauthenticated by design — it creates the tenant. No bearer token.
    let onboard_code = code("RLSON");
    let body = format!(
        r#"{{"code":"{onboard_code}","legal_name":"PT Rls Onboard","base_currency":"IDR"}}"#
    );
    let (status, resp) =
        send_with(app(&module(&app_pool).await), "POST", "/companies/onboard", &body, None).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "onboard must pass the branch fence as a non-owner role; got {status}: {resp}"
    );

    // The head-office branch landed and belongs to the company it created.
    let branch_company: Uuid = sqlx::query_scalar(
        "SELECT b.company_id FROM organization.branches b \
         JOIN organization.companies c ON c.id = b.company_id \
         WHERE c.code = $1",
    )
    .bind(&onboard_code)
    .fetch_one(&admin)
    .await
    .expect("head-office branch row for the onboarded company");
    let company: Uuid = sqlx::query_scalar("SELECT id FROM organization.companies WHERE code = $1")
        .bind(&onboard_code)
        .fetch_one(&admin)
        .await
        .unwrap();
    assert_eq!(branch_company, company, "the head-office branch must sit in its own company");
}
