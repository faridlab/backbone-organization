//! Integration cases for the company operational hierarchy read.
//!
//! Proves `HierarchyService::company_hierarchy` assembles the Company → Branches →
//! Departments tree from real rows: head office first, departments nested by `parent_id`,
//! and partitioned under their `branch_id` (company-level where null). Service-level (not
//! route-level) — the handler is a thin DTO wrapper over this, and this matches the
//! golden-cases pattern. Requires DATABASE_URL (defaults to local dev Postgres on :5433).
//!
//! IGH-1  full tree shape for a seeded company.
//! IGH-2  unknown company → NotFound.

use sqlx::PgPool;
use uuid::Uuid;

use backbone_organization::application::service::{
    HierarchyError, HierarchyService, OnboardRequest, OnboardingService,
};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/backbone_organization".to_string()
    });
    PgPool::connect(&url).await.unwrap()
}

fn unique_code(prefix: &str) -> String {
    format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..8])
}

fn unique_npwp() -> String {
    let hex = Uuid::new_v4().simple().to_string();
    let digits: String = hex.chars().filter(|c| c.is_ascii_digit()).take(15).collect();
    format!("{digits:0<15}")
}

/// Insert a live branch under `company_id` (test pool runs as owner, bypassing RLS).
async fn insert_branch(
    pool: &PgPool,
    company_id: Uuid,
    code: &str,
    name: &str,
    branch_type: &str,
    is_head_office: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organization.branches \
         (id, company_id, code, name, branch_type, is_head_office, status) \
         VALUES ($1, $2, $3, $4, $5::branch_type, $6, 'active'::org_status)",
    )
    .bind(id)
    .bind(company_id)
    .bind(code)
    .bind(name)
    .bind(branch_type)
    .bind(is_head_office)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Insert a live department under `company_id`.
async fn insert_dept(
    pool: &PgPool,
    company_id: Uuid,
    code: &str,
    name: &str,
    parent_id: Option<Uuid>,
    branch_id: Option<Uuid>,
    level: i32,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organization.departments \
         (id, company_id, code, name, parent_id, branch_id, level, is_group, sort_order, status) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,FALSE,0,'active'::org_status)",
    )
    .bind(id)
    .bind(company_id)
    .bind(code)
    .bind(name)
    .bind(parent_id)
    .bind(branch_id)
    .bind(level)
    .execute(pool)
    .await
    .unwrap();
    id
}

// ── IGH-1: full tree shape ──
#[tokio::test]
async fn hierarchy_returns_company_branches_and_department_tree() {
    let pool = pool().await;

    // Seed a company + its head-office branch atomically.
    let onboard = OnboardingService::new(pool.clone());
    let code = unique_code("ACME");
    let mut req = OnboardRequest::new(&code, "PT Acme Indonesia");
    req.npwp = Some(unique_npwp());
    let onboarded = onboard.onboard(req).await.expect("onboard should succeed");
    let company_id = onboarded.company_id;
    let hq_branch_id = onboarded.hq_branch_id;

    // A second, non-HQ branch.
    let regional_id =
        insert_branch(&pool, company_id, "JKT", "Jakarta Branch", "branch", false).await;

    // Company-level department tree: root → child.
    let co_root =
        insert_dept(&pool, company_id, "OPS", "Operations", None, None, 0).await;
    let _co_child =
        insert_dept(&pool, company_id, "OPS-ENG", "Engineering", Some(co_root), None, 1).await;

    // A department under the regional branch.
    let _reg_dept =
        insert_dept(&pool, company_id, "JKT-SALES", "Jakarta Sales", None, Some(regional_id), 0)
            .await;

    let svc = HierarchyService::new(pool.clone());
    let h = svc.company_hierarchy(company_id).await.expect("hierarchy should resolve");

    // Company node.
    assert_eq!(h.company.code, code);
    assert_eq!(h.company.entity_type, "pt");

    // Branches: HQ first, then regional. Both present.
    assert_eq!(h.branches.len(), 2, "HQ + one regional branch");
    assert!(h.branches[0].is_head_office, "head office sorts first");
    assert_eq!(h.branches[0].id, hq_branch_id);
    let regional = h
        .branches
        .iter()
        .find(|b| b.id == regional_id)
        .expect("regional branch present");
    assert_eq!(regional.departments.len(), 1, "regional branch owns its dept");
    assert_eq!(regional.departments[0].code, "JKT-SALES");

    // HQ branch has no departments assigned in this seed.
    let hq = h.branches.iter().find(|b| b.is_head_office).unwrap();
    assert!(hq.departments.is_empty(), "no depts assigned to HQ in this seed");

    // Company-level department tree: one root (OPS) with one child (OPS-ENG).
    assert_eq!(h.departments.len(), 1, "one company-level root");
    assert_eq!(h.departments[0].code, "OPS");
    assert_eq!(h.departments[0].children.len(), 1);
    assert_eq!(h.departments[0].children[0].code, "OPS-ENG");

    // Cleanup.
    sqlx::query("DELETE FROM organization.departments WHERE company_id=$1").bind(company_id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM organization.branches WHERE company_id=$1").bind(company_id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM organization.companies WHERE id=$1").bind(company_id).execute(&pool).await.unwrap();
}

// ── IGH-2: unknown company → NotFound ──
#[tokio::test]
async fn hierarchy_unknown_company_is_not_found() {
    let pool = pool().await;
    let svc = HierarchyService::new(pool.clone());
    let unknown = Uuid::new_v4();
    let err = svc.company_hierarchy(unknown).await.expect_err("unknown company should 404");
    assert!(matches!(err, HierarchyError::NotFound), "expected NotFound, got {err:?}");
}
