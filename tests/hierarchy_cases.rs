//! Integration cases for the company operational hierarchy read.
//!
//! Proves `HierarchyService::company_hierarchy` assembles the Company → Branches →
//! Departments tree from real rows: head office first, departments nested by `parent_id`,
//! grouped under their `branch_id` (a department with no branch does not appear —
//! branch membership is the only placement, ADR-0029). Service-level (not route-level) —
//! the handler is a thin DTO wrapper over this, and this matches the golden-cases pattern.
//! Requires DATABASE_URL (defaults to local dev Postgres on :5433).
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

/// Insert a live branch and its org-units node (id = branch id, parent the company node) —
/// the node is what places the branch in the subtree the hierarchy read walks
/// (test pool runs as owner, bypassing RLS).
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
         (id, code, name, branch_type, is_head_office, status) \
         VALUES ($1, $2, $3, $4::branch_type, $5, 'active'::org_status)",
    )
    .bind(id)
    .bind(code)
    .bind(name)
    .bind(branch_type)
    .bind(is_head_office)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO organization.org_units (id, kind, parent_id, code, name) \
         VALUES ($1, 'branch', $2, $3, $4)",
    )
    .bind(id)
    .bind(company_id)
    .bind(code)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Insert a live department. No tenant axis and no node — a department's placement IS its
/// optional `branch_id` (ADR-0029).
async fn insert_dept(
    pool: &PgPool,
    code: &str,
    name: &str,
    parent_id: Option<Uuid>,
    branch_id: Option<Uuid>,
    level: i32,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organization.departments \
         (id, code, name, parent_id, branch_id, level, is_group, sort_order, status) \
         VALUES ($1,$2,$3,$4,$5,$6,FALSE,0,'active'::org_status)",
    )
    .bind(id)
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

    // Seed a company + its head-office branch (and both nodes) atomically.
    let onboard = OnboardingService::new(pool.clone());
    let code = unique_code("ACME");
    let mut req = OnboardRequest::new(&code, "PT Acme Indonesia");
    req.npwp = Some(unique_npwp());
    let onboarded = onboard.onboard(req).await.expect("onboard should succeed");
    let company_id = onboarded.company_id;
    let hq_branch_id = onboarded.hq_branch_id;

    // A second, non-HQ branch (row + node).
    let regional_id =
        insert_branch(&pool, company_id, "JKT", "Jakarta Branch", "branch", false).await;

    // A department tree under the HQ branch: root → child.
    let hq_root =
        insert_dept(&pool, "OPS", "Operations", None, Some(hq_branch_id), 0).await;
    let _hq_child =
        insert_dept(&pool, "OPS-ENG", "Engineering", Some(hq_root), Some(hq_branch_id), 1).await;

    // A department under the regional branch, and one with NO branch (must not appear).
    let _reg_dept =
        insert_dept(&pool, "JKT-SALES", "Jakarta Sales", None, Some(regional_id), 0).await;
    let _branchless =
        insert_dept(&pool, "LOOSE", "Unanchored", None, None, 0).await;

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

    // HQ branch owns the OPS forest: one root with one child.
    let hq = h.branches.iter().find(|b| b.is_head_office).unwrap();
    assert_eq!(hq.departments.len(), 1, "one OPS root under HQ");
    assert_eq!(hq.departments[0].code, "OPS");
    assert_eq!(hq.departments[0].children.len(), 1);
    assert_eq!(hq.departments[0].children[0].code, "OPS-ENG");

    // The branchless department does not appear anywhere — placement IS the branch.
    let seen: Vec<&String> = h
        .branches
        .iter()
        .flat_map(|b| b.departments.iter().map(|d| &d.code))
        .collect();
    assert!(!seen.iter().any(|c| c.as_str() == "LOOSE"), "branchless dept must be invisible");

    // Cleanup: departments by branch, then the subtree nodes, then the rows, then the company.
    sqlx::query("DELETE FROM organization.departments WHERE branch_id = ANY($1)")
        .bind(vec![hq_branch_id, regional_id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM organization.departments WHERE code = 'LOOSE'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM organization.org_units WHERE id IN \
         (SELECT organization.org_unit_subtree(ARRAY[$1]))",
    )
    .bind(company_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM organization.branches WHERE id = ANY($1)")
        .bind(vec![hq_branch_id, regional_id])
        .execute(&pool)
        .await
        .unwrap();
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
