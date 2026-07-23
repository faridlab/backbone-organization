//! Company hierarchy REST handler — hand-authored (user-owned; never regenerated).
//!
//! Exposes one non-CRUD read endpoint:
//!   GET /companies/{id}/hierarchy → the company, its branches (HQ flagged), and the
//!   full department tree nested under each branch (and company-level where branch_id
//!   is null). Assembled by [`HierarchyService`] from three repository reads.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::application::service::{
    BranchHierarchy, CompanyHierarchy, CompanyInfo, DepartmentNode, HierarchyError,
    HierarchyService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompanyDto {
    id: Uuid,
    code: String,
    legal_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trade_name: Option<String>,
    entity_type: String,
    base_currency: String,
    status: String,
}

impl From<CompanyInfo> for CompanyDto {
    fn from(c: CompanyInfo) -> Self {
        Self {
            id: c.id,
            code: c.code,
            legal_name: c.legal_name,
            trade_name: c.trade_name,
            entity_type: c.entity_type,
            base_currency: c.base_currency,
            status: c.status,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DepartmentDto {
    id: Uuid,
    code: String,
    name: String,
    parent_id: Option<Uuid>,
    branch_id: Option<Uuid>,
    level: i32,
    is_group: bool,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<DepartmentDto>,
}

impl From<DepartmentNode> for DepartmentDto {
    fn from(n: DepartmentNode) -> Self {
        Self {
            id: n.id,
            code: n.code,
            name: n.name,
            parent_id: n.parent_id,
            branch_id: n.branch_id,
            level: n.level,
            is_group: n.is_group,
            status: n.status,
            children: n.children.into_iter().map(DepartmentDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BranchDto {
    id: Uuid,
    code: String,
    name: String,
    branch_type: String,
    is_head_office: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    departments: Vec<DepartmentDto>,
}

impl From<BranchHierarchy> for BranchDto {
    fn from(b: BranchHierarchy) -> Self {
        Self {
            id: b.id,
            code: b.code,
            name: b.name,
            branch_type: b.branch_type,
            is_head_office: b.is_head_office,
            city: b.city,
            status: b.status,
            departments: b.departments.into_iter().map(DepartmentDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyResponse {
    company: CompanyDto,
    branches: Vec<BranchDto>,
    /// Company-level departments (`branch_id` null). Omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    departments: Vec<DepartmentDto>,
}

impl From<CompanyHierarchy> for HierarchyResponse {
    fn from(h: CompanyHierarchy) -> Self {
        Self {
            company: h.company.into(),
            branches: h.branches.into_iter().map(BranchDto::from).collect(),
            departments: h.departments.into_iter().map(DepartmentDto::from).collect(),
        }
    }
}

/// `GET /companies/{id}/hierarchy` — the company's operational org chart.
async fn company_hierarchy(
    State(service): State<Arc<HierarchyService>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match service.company_hierarchy(id).await {
        Ok(hierarchy) => (StatusCode::OK, Json(HierarchyResponse::from(hierarchy))).into_response(),
        Err(err) => {
            let status = StatusCode::from_u16(err.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (
                status,
                Json(ErrorBody {
                    error: err.code(),
                    message: err.to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Hierarchy read route. Merge onto a guarded/read composition.
pub fn create_hierarchy_routes(service: Arc<HierarchyService>) -> Router {
    Router::new()
        .route("/companies/{id}/hierarchy", get(company_hierarchy))
        .with_state(service)
}
