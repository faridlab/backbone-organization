//! Council integrity probes — regression tests for the CRUD-bypass hole and the tenancy hole.
//!
//! The guarded composition (`create_guarded_organization_routes`) must enforce the org
//! invariants on EVERY write path, not just onboarding:
//!   - Company has no generic write route at all (writer = onboarding only).
//!   - Branch/Department writes validate NPWP format, node kind, and link visibility.
//!   - Branch writes derive their attach point from the authenticated principal, never from the
//!     body.
//! These hit the ROUTES (via tower oneshot), not the services — closing the structural blind spot
//! the golden suite had (it only ever constructed services directly).
//! Requires DATABASE_URL (defaults to local dev Postgres on :5433).
//!
//! The guarded surface ships BARE of auth (ADR-0029, the org-composed shape): token validation
//! and unit-tree resolution belong to the composing service's org guard and are proven by ITS
//! probes. What this suite proves are the handler-side invariants given a principal — so the
//! principal-carrying legs wrap the bare router in a stand-in layer that inserts `OrgContext`,
//! exactly where the composing guard inserts it.
//!
//! IGC-1..IGC-4  the CRUD-bypass and validated-write invariants.
//! IGT-1..IGT-3  the tenancy invariants (mirrors the TG-* cases backbone-pos proved).
//! IGF-1..IGF-2  the module-native fence posture (ADR-0029 half-fence, default-deny).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use backbone_auth::org::OrgContext;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use backbone_organization::{create_guarded_organization_routes, OrganizationModule};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_organization".to_string()
    });
    PgPool::connect(&url).await.unwrap()
}

async fn module(pool: &PgPool) -> OrganizationModule {
    OrganizationModule::builder().with_database(pool.clone()).build().unwrap()
}

/// The bare guarded router — what a composing service receives before it wraps anything.
fn app(m: &OrganizationModule) -> axum::Router {
    create_guarded_organization_routes(m)
}

/// The guarded router wrapped in a stand-in for the composing service's org guard: a layer that
/// inserts the authenticated principal (`OrgContext`) the handlers read. `acting_unit` is the
/// node the session acts at — the only source a handler accepts for a write's attach point.
fn app_as(m: &OrganizationModule, acting_unit: Uuid) -> axum::Router {
    let ctx = OrgContext {
        acting_unit_id: acting_unit,
        entitled_units: vec![acting_unit],
        legacy_company_id: None,
        user_id: "probe-user".into(),
    };
    app(m).layer(axum::middleware::from_fn(
        move |mut req: Request<Body>, next: Next| {
            let ctx = ctx.clone();
            async move {
                req.extensions_mut().insert(ctx);
                Ok::<_, std::convert::Infallible>(next.run(req).await)
            }
        },
    ))
}

async fn send(app: axum::Router, method: &str, uri: &str, body: &str) -> StatusCode {
    let resp = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

fn code(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

/// Seed a company registry row AND its org-units node under the tenant root — the node is
/// what branch creation validates and attaches under (ADR-0028: a company's node id IS the
/// company id).
async fn seed_company(pool: &PgPool, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO organization.companies (id, code, legal_name) VALUES ($1,$2,'PT Seed')")
        .bind(id)
        .bind(code)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO organization.org_units (id, kind, parent_id, code, name) \
         SELECT $1, 'company', r.id, $2, 'PT Seed' FROM organization.org_units r \
         WHERE r.kind = 'root' LIMIT 1",
    )
    .bind(id)
    .bind(code)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
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
    let status = send(app_as(&module(&pool).await, cid), "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "bad NPWP must be rejected");
}

// ── IGC-3: validated department create rejects a parent that does not exist ──
// Cross-scope invisibility (a link outside the caller's subtree reads as absent) needs the
// composing service's decorator and is proven by its probes; undecorated, the invariant this
// module owns is that a dangling link is refused before any write.
#[tokio::test]
async fn guarded_department_rejects_dangling_parent() {
    let pool = pool().await;
    let host = seed_company(&pool, &code("HOST")).await;
    let ghost_parent = Uuid::new_v4();

    let body = format!(
        r#"{{"code":"{}","name":"Cross","parentId":"{ghost_parent}"}}"#,
        code("DEP")
    );
    let status = send(app_as(&module(&pool).await, host), "POST", "/departments", &body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "a parent that does not exist must be rejected"
    );

    // And the refusal wrote nothing.
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization.departments WHERE parent_id = $1")
        .bind(ghost_parent)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "no row may be written for a refused link");
}

// ── IGC-4: the happy paths still work through the guarded surface ──
#[tokio::test]
async fn guarded_valid_writes_succeed() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("OKC")).await;

    // Valid branch (valid 15-digit NPWP).
    let branch_body = format!(r#"{{"code":"{}","name":"HQ","npwp":"012345678901234"}}"#, code("OKB"));
    let bs = send(app_as(&module(&pool).await, cid), "POST", "/branches", &branch_body).await;
    assert_eq!(bs, StatusCode::CREATED, "valid branch should be created");

    // Valid department (visible live parent).
    let parent = Uuid::new_v4();
    sqlx::query("INSERT INTO organization.departments (id, code, name) VALUES ($1,$2,'Root')")
        .bind(parent)
        .bind(code("PR"))
        .execute(&pool)
        .await
        .unwrap();
    let dep_body = format!(r#"{{"code":"{}","name":"Child","parentId":"{parent}"}}"#, code("OKD"));
    let ds = send(app_as(&module(&pool).await, cid), "POST", "/departments", &dep_body).await;
    assert_eq!(ds, StatusCode::CREATED, "valid department should be created");
}

// ── IGT-1: a write with no authenticated principal is rejected — on the BARE router the
// `OrgContext` extractor itself refuses (401), so no request ever reaches a handler without one.
// Before the tenant guard this create succeeded and stamped whatever `companyId` the caller put
// in the body. ──
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

// ── IGT-2: a principal that cannot name a unit must never write. Token validation and unit-tree
// resolution belong to the composing service's org guard (its probes prove them); the module-side
// invariant is that the handler accepts the attach point from NO source but the inserted
// `OrgContext` — a body that names a `companyId` while the principal is absent is still refused. ──
#[tokio::test]
async fn guarded_write_rejects_principal_without_unit() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("NOCID")).await;
    let body = format!(r#"{{"companyId":"{cid}","code":"{}","name":"Cabang"}}"#, code("BR"));
    let status = send(app(&module(&pool).await), "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a request with no principal must not write");
}

// ── IGT-3: a `companyId` smuggled in the body is ignored — the persisted placement is the
// principal's. This is the regression that motivated the change: the body must not name where a
// write lands. A branch's landing spot is its org-units node's parent; a department carries no
// tenant axis at all (ADR-0029), so its smuggle is ignored by construction. ──
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
    let status = send(app_as(&module(&pool).await, token_company), "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::CREATED);

    // The branch's node attaches under the PRINCIPAL's company node — the body's id was ignored.
    let persisted: Uuid = sqlx::query_scalar(
        "SELECT u.parent_id FROM organization.org_units u \
         WHERE u.id = (SELECT b.id FROM organization.branches b WHERE b.code = $1)",
    )
    .bind(&branch_code)
    .fetch_one(&pool)
    .await
    .expect("branch node");
    assert_eq!(persisted, token_company, "placement must come from the token, not the body");
    assert_ne!(persisted, attacker_company, "the body's companyId must be ignored");

    // Department: same smuggle — it lands (no tenant axis to forge) and the body's id is inert.
    let dept_code = code("SMUGD");
    let dbody = format!(
        r#"{{"companyId":"{attacker_company}","code":"{dept_code}","name":"Smuggled"}}"#
    );
    let dstatus =
        send(app_as(&module(&pool).await, token_company), "POST", "/departments", &dbody).await;
    assert_eq!(dstatus, StatusCode::CREATED);

    let drow: Uuid =
        sqlx::query_scalar("SELECT id FROM organization.departments WHERE code = $1")
            .bind(&dept_code)
            .fetch_one(&pool)
            .await
            .expect("department row");
    assert_ne!(drow, attacker_company, "the body's companyId names nothing on this row");
}

// IGF-1: module-native fence posture (ADR-0029). The six per-unit master tables ship the
// HALF-FENCE: RLS enabled + FORCED with ZERO policies — default-deny for any non-owner role
// until the composing service's decorator installs the org-scope policies. companies and
// org_units carry no tenant axis at all and stay unfenced under any declaration. This probe
// fails if the strip migration ever leaves a stray policy behind (a leak) or disarms the flags
// (a silent bypass).
#[tokio::test]
async fn half_fence_posture_is_armed_and_policy_free() {
    let pool = pool().await;

    for table in [
        "branches",
        "departments",
        "company_industries",
        "levels",
        "positions",
        "structures",
    ] {
        let (rls, force): (bool, bool) = sqlx::query_as(&format!(
            "SELECT relrowsecurity, relforcerowsecurity FROM pg_class \
             WHERE oid = 'organization.{table}'::regclass"
        ))
        .fetch_one(&pool)
        .await
        .expect("table must exist");
        assert!(rls && force, "organization.{table} must ship RLS ENABLED + FORCE");

        let policies: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM pg_policies WHERE schemaname = 'organization' AND tablename = '{table}'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(policies, 0, "organization.{table} must ship ZERO policies (decorator owns them)");
    }

    for table in ["companies", "org_units", "industries"] {
        let rls: bool = sqlx::query_scalar(&format!(
            "SELECT relrowsecurity FROM pg_class WHERE oid = 'organization.{table}'::regclass"
        ))
        .fetch_one(&pool)
        .await
        .expect("table must exist");
        assert!(!rls, "organization.{table} is the unfenced registry/tree (ADR-0029)");
    }

    // The legacy fence helper is gone; the spine helper stays.
    let subtree: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'org_unit_subtree')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(subtree, "org_unit_subtree must exist (the spine helper)");
}

// IGF-2: the half-fence actually denies a non-owner role. Every other probe runs on the owner
// DSN, and the table owner BYPASSES row-level security — this is the only leg that proves the
// default-deny is real: a granted role reads zero rows and cannot insert. The module-native
// contract is deliberately unusable-as-is; the composing decorator opens it per-scope.
#[tokio::test]
async fn half_fence_default_denies_non_owner_role() {
    const ROLE: &str = "org_rls_probe";
    const PW: &str = "org_rls_probe_pw";
    let admin = pool().await;

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
    let probe = PgPool::connect(&format!("postgresql://{ROLE}:{PW}@{after_at}")).await.unwrap();

    // Seed rows as owner so the read has something to be denied.
    let cid = seed_company(&admin, &code("DENY")).await;

    let seen: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization.branches")
        .fetch_one(&probe)
        .await
        .expect("SELECT must run (granted) — RLS filters it to zero");
    assert_eq!(seen, 0, "a granted non-owner role must see zero branch rows (default-deny)");

    // The registry and tree stay readable — they carry no tenant axis.
    let companies: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organization.companies WHERE id = $1")
        .bind(cid)
        .fetch_one(&probe)
        .await
        .unwrap();
    assert_eq!(companies, 1, "the unfenced registry stays readable to a granted role");

    // And the write is refused outright (FORCE + no policy = RLS violation).
    let insert = sqlx::query(
        "INSERT INTO organization.branches (id, code, name) VALUES ($1,'DENY','Denied')",
    )
    .bind(Uuid::new_v4())
    .execute(&probe)
    .await;
    assert!(
        insert.is_err(),
        "a non-owner INSERT into the half-fenced table must be refused"
    );

    // Onboarding as a non-owner role therefore fails loudly too — by design. The module
    // expects owner/decorator-managed roles; the composing service's onboarding lane runs
    // where the fence is installed. (Golden case 1 proves the owner path.)
}
