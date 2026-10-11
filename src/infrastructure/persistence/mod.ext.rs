// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

// The hand-written validated-write SQL's parameter types (see the three repositories, all declared
// `user_owned` in metaphor.codegen.yaml).
pub use branch_repository::{NewBranchRow, NewHeadOfficeBranchRow};
pub use company_repository::NewCompanyRow;
pub use department_repository::NewDepartmentRow;
// Hierarchy-read row shapes (company/branch/department).
pub use branch_repository::BranchHierarchyRow;
pub use company_repository::CompanyHierarchyRow;
pub use department_repository::DepartmentHierarchyRow;
// Re-bind the caller's ambient org scope onto a transaction opened on the plain pool — the
// scope is task-local and a fresh pool transaction carries none of it. With no ambient scope
// (standalone deployment, onboarding, jobs) the transaction stays plain: the module is
// tenant-agnostic and the composing service's tenancy decorator owns isolation (ADR-0029).
// Same convention as the backbone-pos / backbone-promo / backbone-livechat write paths.
pub(crate) async fn relay_ambient_scope(conn: &mut sqlx::PgConnection) -> Result<(), sqlx::Error> {
    if let Some(scope) = backbone_orm::org_scope::current_org_scope() {
        backbone_orm::org_scope::bind_org_scope_on(conn, &scope).await?;
    }
    Ok(())
}
