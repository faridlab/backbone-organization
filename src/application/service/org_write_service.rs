//! Validated write path for Branch and Department — hand-authored (user-owned).
//!
//! Closes the CRUD-bypass the council flagged: the generated 12-endpoint CRUD writes rows through
//! `GenericCrudService` with NO domain validation, so a well-formed request can create a branch
//! with a malformed NPWP, or a department whose `parent_id`/`branch_id` belongs to a *different*
//! company — corrupting the org dimension every downstream module trusts.
//!
//! `OrganizationModule` mounts these validated writers (plus onboarding for Company) instead of
//! the raw CRUD writers. Company has no validated CRUD writer at all: its only writer is
//! `OnboardingService` (a company must be born with a head-office branch).

use backbone_orm::company_scope;
use sqlx::PgPool;
use uuid::Uuid;

use super::onboarding_service::validate_npwp;

#[derive(Debug)]
pub enum OrgWriteError {
    InvalidNpwp(String),
    CompanyNotFound(Uuid),
    ParentNotFound(Uuid),
    ParentDifferentCompany,
    BranchNotFound(Uuid),
    BranchDifferentCompany,
    SelfParent,
    Db(sqlx::Error),
}

impl OrgWriteError {
    pub fn code(&self) -> &'static str {
        match self {
            OrgWriteError::InvalidNpwp(_) => "invalid_npwp",
            OrgWriteError::CompanyNotFound(_) => "company_not_found",
            OrgWriteError::ParentNotFound(_) => "parent_not_found",
            OrgWriteError::ParentDifferentCompany => "parent_different_company",
            OrgWriteError::BranchNotFound(_) => "branch_not_found",
            OrgWriteError::BranchDifferentCompany => "branch_different_company",
            OrgWriteError::SelfParent => "self_parent",
            OrgWriteError::Db(_) => "internal_error",
        }
    }
    pub fn http_status(&self) -> u16 {
        match self {
            OrgWriteError::Db(_) => 500,
            _ => 422,
        }
    }
}
impl std::fmt::Display for OrgWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())?;
        match self {
            OrgWriteError::InvalidNpwp(v) => write!(f, ": {v}"),
            OrgWriteError::CompanyNotFound(id)
            | OrgWriteError::ParentNotFound(id)
            | OrgWriteError::BranchNotFound(id) => write!(f, ": {id}"),
            _ => Ok(()),
        }
    }
}
impl std::error::Error for OrgWriteError {}
impl From<sqlx::Error> for OrgWriteError {
    fn from(e: sqlx::Error) -> Self {
        OrgWriteError::Db(e)
    }
}

#[derive(Debug, Clone)]
pub struct NewBranch {
    pub company_id: Uuid,
    pub code: String,
    pub name: String,
    pub branch_type: Option<String>,
    pub is_head_office: bool,
    pub npwp: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewDepartment {
    pub company_id: Uuid,
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub is_group: bool,
    pub manager_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct OrgWriteService {
    db_pool: PgPool,
}

impl OrgWriteService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    async fn company_exists(&self, id: Uuid) -> Result<bool, OrgWriteError> {
        // RLS scope (ADR-0008): the id being probed IS the company — fence the probe to it.
        let exists_q = sqlx::query_scalar(
            "SELECT id FROM organization.companies WHERE id=$1 AND (metadata->>'deleted_at') IS NULL",
        )
        .bind(id);
        let found: Option<Uuid> = company_scope::with_company_scope(
            Some(id),
            company_scope::fetch_optional_scalar_scoped(&self.db_pool, exists_q),
        ).await?;
        Ok(found.is_some())
    }

    pub async fn create_branch(&self, b: NewBranch) -> Result<Uuid, OrgWriteError> {
        if let Some(n) = &b.npwp {
            if !validate_npwp(n) {
                return Err(OrgWriteError::InvalidNpwp(n.clone()));
            }
        }
        if !self.company_exists(b.company_id).await? {
            return Err(OrgWriteError::CompanyNotFound(b.company_id));
        }
        let id = Uuid::new_v4();
        let branch_type = b.branch_type.clone().unwrap_or_else(|| "branch".to_string());
        // RLS scope (ADR-0008), DTO-company pattern: the company is on the DTO — the INSERT's WITH
        // CHECK needs `app.company_id` bound or it is rejected under the app role.
        let insert_q = sqlx::query(
            r#"INSERT INTO organization.branches
                (id, company_id, code, name, branch_type, is_head_office, npwp, email, phone, address, status)
               VALUES ($1,$2,$3,$4,$5::branch_type,$6,$7,$8,$9,$10,'active'::org_status)"#,
        )
        .bind(id)
        .bind(b.company_id)
        .bind(&b.code)
        .bind(&b.name)
        .bind(&branch_type)
        .bind(b.is_head_office)
        .bind(&b.npwp)
        .bind(&b.email)
        .bind(&b.phone)
        .bind(&b.address);
        company_scope::with_company_scope(
            Some(b.company_id),
            company_scope::execute_scoped(&self.db_pool, insert_q),
        ).await?;
        Ok(id)
    }

    /// Validate that `parent_id` / `branch_id` (if present) exist AND belong to `company_id`.
    async fn validate_dept_links(
        &self,
        company_id: Uuid,
        parent_id: Option<Uuid>,
        branch_id: Option<Uuid>,
        self_id: Option<Uuid>,
    ) -> Result<(), OrgWriteError> {
        if let Some(pid) = parent_id {
            if Some(pid) == self_id {
                return Err(OrgWriteError::SelfParent);
            }
            // RLS scope (ADR-0008), param-company pattern: fence the link probe to the caller's
            // company. The explicit `c != company_id` check below stays as defense-in-depth.
            let parent_q = sqlx::query_scalar(
                "SELECT company_id FROM organization.departments WHERE id=$1 AND (metadata->>'deleted_at') IS NULL",
            )
            .bind(pid);
            let owner: Option<Uuid> = company_scope::with_company_scope(
                Some(company_id),
                company_scope::fetch_optional_scalar_scoped(&self.db_pool, parent_q),
            ).await?;
            match owner {
                None => return Err(OrgWriteError::ParentNotFound(pid)),
                Some(c) if c != company_id => return Err(OrgWriteError::ParentDifferentCompany),
                _ => {}
            }
        }
        if let Some(bid) = branch_id {
            let branch_q = sqlx::query_scalar(
                "SELECT company_id FROM organization.branches WHERE id=$1 AND (metadata->>'deleted_at') IS NULL",
            )
            .bind(bid);
            let owner: Option<Uuid> = company_scope::with_company_scope(
                Some(company_id),
                company_scope::fetch_optional_scalar_scoped(&self.db_pool, branch_q),
            ).await?;
            match owner {
                None => return Err(OrgWriteError::BranchNotFound(bid)),
                Some(c) if c != company_id => return Err(OrgWriteError::BranchDifferentCompany),
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn create_department(&self, d: NewDepartment) -> Result<Uuid, OrgWriteError> {
        if !self.company_exists(d.company_id).await? {
            return Err(OrgWriteError::CompanyNotFound(d.company_id));
        }
        self.validate_dept_links(d.company_id, d.parent_id, d.branch_id, None)
            .await?;
        let id = Uuid::new_v4();
        // RLS scope (ADR-0008), DTO-company pattern: bind the DTO's company so the INSERT's WITH CHECK
        // passes under the app role.
        let insert_q = sqlx::query(
            r#"INSERT INTO organization.departments
                (id, company_id, code, name, parent_id, branch_id, is_group, manager_id, status)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'active'::org_status)"#,
        )
        .bind(id)
        .bind(d.company_id)
        .bind(&d.code)
        .bind(&d.name)
        .bind(d.parent_id)
        .bind(d.branch_id)
        .bind(d.is_group)
        .bind(d.manager_id);
        company_scope::with_company_scope(
            Some(d.company_id),
            company_scope::execute_scoped(&self.db_pool, insert_q),
        ).await?;
        Ok(id)
    }

    /// Re-point a department's `parent_id` / `branch_id`, enforcing same-company + no self-parent.
    /// Only the invariant-bearing links are mutable here; other fields use generic CRUD PATCH is
    /// intentionally NOT exposed (see guarded_routes composition).
    pub async fn repoint_department(
        &self,
        id: Uuid,
        parent_id: Option<Uuid>,
        branch_id: Option<Uuid>,
    ) -> Result<(), OrgWriteError> {
        // RLS scope (ADR-0008), ID-only pattern: identified by department id alone — no company
        // argument. This read rides the REQUEST-dedicated connection (established by `company_auth`),
        // whose `app.company_id` fences it, so another company's department isn't found. Having read
        // the owning company, we fence the update to it explicitly below.
        let owner_q = sqlx::query_scalar(
            "SELECT company_id FROM organization.departments WHERE id=$1 AND (metadata->>'deleted_at') IS NULL",
        )
        .bind(id);
        let company_id: Option<Uuid> =
            company_scope::fetch_optional_scalar_scoped(&self.db_pool, owner_q).await?;
        let company_id = company_id.ok_or(OrgWriteError::ParentNotFound(id))?;
        self.validate_dept_links(company_id, parent_id, branch_id, Some(id))
            .await?;
        let update_q =
            sqlx::query("UPDATE organization.departments SET parent_id=$2, branch_id=$3 WHERE id=$1")
                .bind(id)
                .bind(parent_id)
                .bind(branch_id);
        company_scope::with_company_scope(
            Some(company_id),
            company_scope::execute_scoped(&self.db_pool, update_q),
        ).await?;
        Ok(())
    }
}
