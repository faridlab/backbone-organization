//! Council integrity probes — regression tests for the CRUD-bypass hole.
//!
//! The guarded composition (`create_guarded_organization_routes`) must enforce the org
//! invariants on EVERY write path, not just onboarding:
//!   - Company has no generic write route at all (writer = onboarding only).
//!   - Branch/Department writes validate NPWP format, company existence, and same-company links.
//! These hit the ROUTES (via tower oneshot), not the services — closing the structural blind spot
//! the golden suite had (it only ever constructed services directly).
//! Requires DATABASE_URL (defaults to local dev Postgres on :5433).

use axum::body::Body;
use axum::http::{Request, StatusCode};
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

async fn send(app: axum::Router, method: &str, uri: &str, body: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
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

// ── Probe 1: the guarded surface mounts NO generic write route for companies ──
#[tokio::test]
async fn guarded_routes_lock_company_writes() {
    let pool = pool().await;
    let body = format!(
        r#"{{"code":"{}","legalName":"PT Bypass","entityType":"pt","baseCurrency":"IDR","fiscalYearStartMonth":1,"country":"ID","isDefault":false,"status":"active"}}"#,
        code("LOCK")
    );
    // POST /companies is not routed in the guarded composition → 405/404, never 201.
    let status = send(create_guarded_organization_routes(&module(&pool).await), "POST", "/companies", &body).await;
    assert!(
        status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
        "guarded routes must not expose generic company create; got {status}"
    );
}

// ── Probe 2: validated branch create rejects a malformed NPWP ──
#[tokio::test]
async fn guarded_branch_rejects_bad_npwp() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("BRC")).await;
    let body = format!(
        r#"{{"companyId":"{cid}","code":"{}","name":"Cabang","npwp":"12345"}}"#,
        code("BR")
    );
    let status = send(create_guarded_organization_routes(&module(&pool).await), "POST", "/branches", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "bad NPWP must be rejected");
}

// ── Probe 3: validated department create rejects a cross-company parent ──
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

    let body = format!(
        r#"{{"companyId":"{host}","code":"{}","name":"Cross","parentId":"{foreign_parent}"}}"#,
        code("DEP")
    );
    let status = send(create_guarded_organization_routes(&module(&pool).await), "POST", "/departments", &body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "cross-company parent must be rejected"
    );
}

// ── Probe 4: the happy paths still work through the guarded surface ──
#[tokio::test]
async fn guarded_valid_writes_succeed() {
    let pool = pool().await;
    let cid = seed_company(&pool, &code("OKC")).await;
    let app = create_guarded_organization_routes(&module(&pool).await);

    // Valid branch (valid 15-digit NPWP).
    let branch_body = format!(
        r#"{{"companyId":"{cid}","code":"{}","name":"HQ","npwp":"012345678901234"}}"#,
        code("OKB")
    );
    let bs = send(app, "POST", "/branches", &branch_body).await;
    assert_eq!(bs, StatusCode::CREATED, "valid branch should be created");

    // Valid department (same-company parent).
    let app2 = create_guarded_organization_routes(&module(&pool).await);
    let parent = Uuid::new_v4();
    sqlx::query("INSERT INTO organization.departments (id, company_id, code, name) VALUES ($1,$2,$3,'Root')")
        .bind(parent)
        .bind(cid)
        .bind(code("PR"))
        .execute(&pool)
        .await
        .unwrap();
    let dep_body = format!(
        r#"{{"companyId":"{cid}","code":"{}","name":"Child","parentId":"{parent}"}}"#,
        code("OKD")
    );
    let ds = send(app2, "POST", "/departments", &dep_body).await;
    assert_eq!(ds, StatusCode::CREATED, "valid department should be created");
}
