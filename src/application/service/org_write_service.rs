//! Validated write path for Branch and Department — hand-authored (user-owned).
//!
//! Closes the CRUD-bypass the council flagged: the generated 12-endpoint CRUD writes rows through
//! `GenericCrudService` with NO domain validation, so a well-formed request can create a branch
//! with a malformed NPWP, or a department whose `parent_id`/`branch_id` points at nothing —
//! corrupting the org dimension every downstream module trusts.
//!
//! `OrganizationModule` mounts these validated writers (plus onboarding for Company) instead of
//! the raw CRUD writers. Company has no validated CRUD writer at all: its only writer is
//! `OnboardingService` (a company must be born with its org-units node and a head-office branch).
//!
//! Isolation (ADR-0029): the module is tenant-agnostic — no company column, no fence. Link
//! validation is EXISTENCE-ONLY: probes ride the request-dedicated connection when the composing
//! service bound a scope, and a link outside the caller's visible subtree is simply absent, so a
//! cross-scope link fails closed without any same-company comparison. Writes open their own
//! transaction on the service pool and re-bind the caller's ambient org scope onto it
//! (`relay_ambient_scope`); a composing decorator's WITH CHECK is what actually fences the row.

use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    relay_ambient_scope, BranchRepository, CompanyRepository, DepartmentRepository, NewBranchRow,
    NewDepartmentRow,
};

use super::onboarding_service::validate_npwp;

#[derive(Debug)]
pub enum OrgWriteError {
    InvalidNpwp(String),
    /// The org-units node a branch was to attach under is absent or not a company/branch node.
    ParentUnitNotFound(Uuid),
    ParentNotFound(Uuid),
    BranchNotFound(Uuid),
    DepartmentNotFound(Uuid),
    SelfParent,
    Db(sqlx::Error),
}

impl OrgWriteError {
    pub fn code(&self) -> &'static str {
        match self {
            OrgWriteError::InvalidNpwp(_) => "invalid_npwp",
            OrgWriteError::ParentUnitNotFound(_) => "parent_unit_not_found",
            OrgWriteError::ParentNotFound(_) => "parent_not_found",
            OrgWriteError::BranchNotFound(_) => "branch_not_found",
            OrgWriteError::DepartmentNotFound(_) => "department_not_found",
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
            OrgWriteError::ParentUnitNotFound(id)
            | OrgWriteError::ParentNotFound(id)
            | OrgWriteError::BranchNotFound(id)
            | OrgWriteError::DepartmentNotFound(id) => write!(f, ": {id}"),
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
    /// The org-units node the branch attaches under — a company node or an ancestor branch
    /// node (validated at creation; the node is minted in the same transaction as the row).
    pub parent_unit: Uuid,
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
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub is_group: bool,
    pub manager_id: Option<Uuid>,
}

pub struct OrgWriteService {
    db_pool: PgPool,
    companies: CompanyRepository,
    branches: BranchRepository,
    departments: DepartmentRepository,
}

/// The repositories are pool handles, so a clone is just a re-wire off the same cloned pool — this
/// stays as cheap as the `#[derive(Clone)]` it replaces (the repo newtypes are not themselves `Clone`).
impl Clone for OrgWriteService {
    fn clone(&self) -> Self {
        Self::new(self.db_pool.clone())
    }
}

impl OrgWriteService {
    pub fn new(db_pool: PgPool) -> Self {
        let companies = CompanyRepository::new(db_pool.clone());
        let branches = BranchRepository::new(db_pool.clone());
        let departments = DepartmentRepository::new(db_pool.clone());
        Self { db_pool, companies, branches, departments }
    }

    /// Create a branch and mint its org-units node as one unit of work.
    ///
    /// The parent unit must be an existing node of kind company or branch — the two kinds a
    /// branch may attach under. The branch row and its node (id = branch id, kind 'branch',
    /// parent the validated node) commit together: a branch can never exist unplaced in the
    /// tree, and the tree can never hold a node without its row.
    pub async fn create_branch(&self, b: NewBranch) -> Result<Uuid, OrgWriteError> {
        if let Some(n) = &b.npwp {
            if !validate_npwp(n) {
                return Err(OrgWriteError::InvalidNpwp(n.clone()));
            }
        }
        match self.companies.find_node_kind(&self.db_pool, b.parent_unit).await? {
            Some(k) if k == "company" || k == "branch" => {}
            _ => return Err(OrgWriteError::ParentUnitNotFound(b.parent_unit)),
        }
        let id = Uuid::new_v4();
        let branch_type = b.branch_type.clone().unwrap_or_else(|| "branch".to_string());
        let row = NewBranchRow {
            id,
            parent_unit: b.parent_unit,
            code: &b.code,
            name: &b.name,
            branch_type: &branch_type,
            is_head_office: b.is_head_office,
            npwp: b.npwp.as_ref(),
            email: b.email.as_ref(),
            phone: b.phone.as_ref(),
            address: b.address.as_ref(),
        };
        let mut tx = self.db_pool.begin().await?;
        relay_ambient_scope(&mut tx).await?;
        self.branches.insert_branch_on(&mut tx, &row).await?;
        self.branches
            .insert_branch_node_on(&mut tx, id, b.parent_unit, &b.code, &b.name)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Validate that `parent_id` / `branch_id` (if present) are visible live rows.
    ///
    /// Existence-only (ADR-0029): the probes ride the request-dedicated connection when the
    /// composing service bound a scope, so a link outside the caller's subtree reads as absent —
    /// fail-closed without a same-company comparison.
    async fn validate_dept_links(
        &self,
        parent_id: Option<Uuid>,
        branch_id: Option<Uuid>,
        self_id: Option<Uuid>,
    ) -> Result<(), OrgWriteError> {
        if let Some(pid) = parent_id {
            if Some(pid) == self_id {
                return Err(OrgWriteError::SelfParent);
            }
            if !self.departments.exists_live(&self.db_pool, pid).await? {
                return Err(OrgWriteError::ParentNotFound(pid));
            }
        }
        if let Some(bid) = branch_id {
            if !self.branches.exists_live(&self.db_pool, bid).await? {
                return Err(OrgWriteError::BranchNotFound(bid));
            }
        }
        Ok(())
    }

    pub async fn create_department(&self, d: NewDepartment) -> Result<Uuid, OrgWriteError> {
        self.validate_dept_links(d.parent_id, d.branch_id, None).await?;
        let id = Uuid::new_v4();
        let mut tx = self.db_pool.begin().await?;
        relay_ambient_scope(&mut tx).await?;
        self.departments
            .insert_department_on(
                &mut tx,
                &NewDepartmentRow {
                    id,
                    code: &d.code,
                    name: &d.name,
                    parent_id: d.parent_id,
                    branch_id: d.branch_id,
                    is_group: d.is_group,
                    manager_id: d.manager_id,
                },
            )
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Re-point a department's `parent_id` / `branch_id`, enforcing visibility + no self-parent.
    /// Only the invariant-bearing links are mutable here; other fields use generic CRUD PATCH is
    /// intentionally NOT exposed (see guarded_routes composition).
    pub async fn repoint_department(
        &self,
        id: Uuid,
        parent_id: Option<Uuid>,
        branch_id: Option<Uuid>,
    ) -> Result<(), OrgWriteError> {
        if !self.departments.exists_live(&self.db_pool, id).await? {
            return Err(OrgWriteError::DepartmentNotFound(id));
        }
        self.validate_dept_links(parent_id, branch_id, Some(id)).await?;
        let mut tx = self.db_pool.begin().await?;
        relay_ambient_scope(&mut tx).await?;
        self.departments
            .repoint_on(&mut tx, id, parent_id, branch_id)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
