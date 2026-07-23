//! Company operational hierarchy — assemble a Company → Branches → Departments
//! tree in one read.
//!
//! Hand-authored (user-owned; see `metaphor.codegen.yaml`). Orchestrates three
//! repository reads and builds the department forest in memory. SQL lives in
//! the repos (4-layer rule); this service only orchestrates and shapes.

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    BranchHierarchyRow, BranchRepository, CompanyHierarchyRow, CompanyRepository,
    DepartmentHierarchyRow, DepartmentRepository,
};

// ---------------------------------------------------------------------------
// Output (domain) types — serializable DTOs live in the handler.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CompanyInfo {
    pub id: Uuid,
    pub code: String,
    pub legal_name: String,
    pub trade_name: Option<String>,
    pub entity_type: String,
    pub base_currency: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DepartmentNode {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub level: i32,
    pub is_group: bool,
    pub status: String,
    pub children: Vec<DepartmentNode>,
}

#[derive(Debug, Clone)]
pub struct BranchHierarchy {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub branch_type: String,
    pub is_head_office: bool,
    pub city: Option<String>,
    pub status: String,
    pub departments: Vec<DepartmentNode>,
}

#[derive(Debug, Clone)]
pub struct CompanyHierarchy {
    pub company: CompanyInfo,
    pub branches: Vec<BranchHierarchy>,
    /// Company-level departments (`branch_id` is null).
    pub departments: Vec<DepartmentNode>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum HierarchyError {
    NotFound,
    Db(sqlx::Error),
}

impl HierarchyError {
    pub fn code(&self) -> &'static str {
        match self {
            HierarchyError::NotFound => "company_not_found",
            HierarchyError::Db(_) => "internal_error",
        }
    }
    pub fn http_status(&self) -> u16 {
        match self {
            HierarchyError::NotFound => 404,
            HierarchyError::Db(_) => 500,
        }
    }
}
impl std::fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HierarchyError::NotFound => write!(f, "company_not_found"),
            HierarchyError::Db(e) => write!(f, "db_error: {e}"),
        }
    }
}
impl std::error::Error for HierarchyError {}
impl From<sqlx::Error> for HierarchyError {
    fn from(e: sqlx::Error) -> Self {
        HierarchyError::Db(e)
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HierarchyService {
    db_pool: PgPool,
    companies: Arc<CompanyRepository>,
    branches: Arc<BranchRepository>,
    departments: Arc<DepartmentRepository>,
}

impl HierarchyService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            companies: Arc::new(CompanyRepository::new(db_pool.clone())),
            branches: Arc::new(BranchRepository::new(db_pool.clone())),
            departments: Arc::new(DepartmentRepository::new(db_pool.clone())),
            db_pool,
        }
    }

    /// Build the operational hierarchy for one company.
    ///
    /// `NotFound` if the company doesn't exist (or isn't visible under the
    /// request's company scope — `find_live_by_id` is company-scoped per
    /// ADR-0008).
    pub async fn company_hierarchy(
        &self,
        company_id: Uuid,
    ) -> Result<CompanyHierarchy, HierarchyError> {
        let company = self
            .companies
            .find_live_by_id(&self.db_pool, company_id)
            .await?
            .ok_or(HierarchyError::NotFound)?;

        let branch_rows = self
            .branches
            .list_live_by_company(&self.db_pool, company_id)
            .await?;
        let dept_rows = self
            .departments
            .list_live_by_company(&self.db_pool, company_id)
            .await?;

        // Partition departments by branch (None → company-level) and build a
        // parent_id forest within each partition.
        let (company_depts, mut branch_depts) = partition_and_build(dept_rows);

        let mut branches = Vec::with_capacity(branch_rows.len());
        for b in branch_rows {
            let departments = branch_depts.remove(&b.id).unwrap_or_default();
            branches.push(BranchHierarchy {
                id: b.id,
                code: b.code,
                name: b.name,
                branch_type: b.branch_type,
                is_head_office: b.is_head_office,
                city: b.city,
                status: b.status,
                departments,
            });
        }

        Ok(CompanyHierarchy {
            company: CompanyInfo {
                id: company.id,
                code: company.code,
                legal_name: company.legal_name,
                trade_name: company.trade_name,
                entity_type: company.entity_type,
                base_currency: company.base_currency,
                status: company.status,
            },
            branches,
            departments: company_depts,
        })
    }
}

// ---------------------------------------------------------------------------
// Pure forest construction (unit-tested).
// ---------------------------------------------------------------------------

/// Partition departments by `branch_id` (None → company-level) and build a
/// `parent_id` forest within each partition. Returns the company-level forest
/// and a map of branch_id → that branch's department forest.
pub(crate) fn partition_and_build(
    rows: Vec<DepartmentHierarchyRow>,
) -> (Vec<DepartmentNode>, HashMap<Uuid, Vec<DepartmentNode>>) {
    let mut company: Vec<DepartmentHierarchyRow> = Vec::new();
    let mut by_branch: HashMap<Uuid, Vec<DepartmentHierarchyRow>> = HashMap::new();
    for r in rows {
        match r.branch_id {
            Some(bid) => by_branch.entry(bid).or_default().push(r),
            None => company.push(r),
        }
    }
    let company_forest = build_forest(company);
    let branch_forests = by_branch
        .into_iter()
        .map(|(bid, rs)| (bid, build_forest(rs)))
        .collect();
    (company_forest, branch_forests)
}

/// Build a `parent_id` forest from rows ordered parents-first (level, then
/// sort_order, then code). A node whose `parent_id` is missing from the set
/// (dangling) is treated as a root.
fn build_forest(rows: Vec<DepartmentHierarchyRow>) -> Vec<DepartmentNode> {
    let mut by_id: HashMap<Uuid, DepartmentHierarchyRow> = HashMap::new();
    let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut roots: Vec<Uuid> = Vec::new();
    for r in rows {
        let id = r.id;
        let parent_present = matches!(r.parent_id, Some(pid) if by_id.contains_key(&pid));
        if parent_present {
            children.entry(r.parent_id.unwrap()).or_default().push(id);
        } else {
            roots.push(id);
        }
        by_id.insert(id, r);
    }
    roots
        .into_iter()
        .map(|id| build_node(id, &by_id, &children))
        .collect()
}

fn build_node(
    id: Uuid,
    by_id: &HashMap<Uuid, DepartmentHierarchyRow>,
    children: &HashMap<Uuid, Vec<Uuid>>,
) -> DepartmentNode {
    let r = &by_id[&id];
    let child_nodes = children
        .get(&id)
        .map(|cs| cs.iter().map(|c| build_node(*c, by_id, children)).collect())
        .unwrap_or_default();
    DepartmentNode {
        id: r.id,
        code: r.code.clone(),
        name: r.name.clone(),
        parent_id: r.parent_id,
        branch_id: r.branch_id,
        level: r.level,
        is_group: r.is_group,
        status: r.status.clone(),
        children: child_nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dept(id: &str, code: &str, parent: Option<&str>, branch: Option<&str>, level: i32) -> DepartmentHierarchyRow {
        DepartmentHierarchyRow {
            id: Uuid::parse_str(id).unwrap(),
            code: code.to_string(),
            name: code.to_string(),
            parent_id: parent.map(|p| Uuid::parse_str(p).unwrap()),
            branch_id: branch.map(|b| Uuid::parse_str(b).unwrap()),
            level,
            is_group: false,
            status: "active".to_string(),
        }
    }

    #[test]
    fn builds_two_level_tree_under_company() {
        // a (root) → b → c ; plus a standalone root d. All company-level (no branch).
        let rows = vec![
            dept("00000000-0000-0000-0000-000000000001", "a", None, None, 0),
            dept("00000000-0000-0000-0000-000000000002", "b", Some("00000000-0000-0000-0000-000000000001"), None, 1),
            dept("00000000-0000-0000-0000-000000000003", "c", Some("00000000-0000-0000-0000-000000000002"), None, 2),
            dept("00000000-0000-0000-0000-000000000004", "d", None, None, 0),
        ];
        let (company, by_branch) = partition_and_build(rows);
        assert!(by_branch.is_empty());
        assert_eq!(company.len(), 2, "two roots (a, d)");
        let a = company.iter().find(|n| n.code == "a").unwrap();
        assert_eq!(a.children.len(), 1);
        assert_eq!(a.children[0].code, "b");
        assert_eq!(a.children[0].children.len(), 1);
        assert_eq!(a.children[0].children[0].code, "c");
    }

    #[test]
    fn partitions_departments_under_their_branch() {
        let hq = "11111111-1111-1111-1111-111111111111";
        let br = "22222222-2222-2222-2222-222222222222";
        let rows = vec![
            dept("00000000-0000-0000-0000-000000000001", "hq-root", None, Some(hq), 0),
            dept("00000000-0000-0000-0000-000000000002", "hq-child", Some("00000000-0000-0000-0000-000000000001"), Some(hq), 1),
            dept("00000000-0000-0000-0000-000000000003", "br-root", None, Some(br), 0),
            dept("00000000-0000-0000-0000-000000000004", "co-root", None, None, 0),
        ];
        let (company, by_branch) = partition_and_build(rows);
        // Company-level: one root.
        assert_eq!(company.len(), 1);
        assert_eq!(company[0].code, "co-root");
        // HQ branch forest: hq-root → hq-child.
        let hq_forest = by_branch.get(&Uuid::parse_str(hq).unwrap()).unwrap();
        assert_eq!(hq_forest.len(), 1);
        assert_eq!(hq_forest[0].code, "hq-root");
        assert_eq!(hq_forest[0].children.len(), 1);
        assert_eq!(hq_forest[0].children[0].code, "hq-child");
        // Branch forest: br-root only.
        let br_forest = by_branch.get(&Uuid::parse_str(br).unwrap()).unwrap();
        assert_eq!(br_forest.len(), 1);
        assert_eq!(br_forest[0].code, "br-root");
    }

    #[test]
    fn dangling_parent_becomes_root() {
        // parent points to a node not in the set → treated as a root, not dropped.
        let rows = vec![dept(
            "00000000-0000-0000-0000-000000000009",
            "orphan",
            Some("ffffffff-ffff-ffff-ffff-ffffffffffff"),
            None,
            1,
        )];
        let (company, _) = partition_and_build(rows);
        assert_eq!(company.len(), 1);
        assert_eq!(company[0].code, "orphan");
    }

    #[test]
    fn empty_input_yields_empty_forests() {
        let (company, by_branch) = partition_and_build(vec![]);
        assert!(company.is_empty());
        assert!(by_branch.is_empty());
    }
}
