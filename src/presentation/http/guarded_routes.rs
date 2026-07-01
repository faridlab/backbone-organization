//! Guarded route composition — the RECOMMENDED way to mount the organization module.
//!
//! Hand-authored (user-owned; see `metaphor.codegen.yaml`). Closes the CRUD-bypass the council
//! flagged: the generated `routes()` exposes full mutable CRUD (POST/PATCH/upsert/bulk) on every
//! entity, backed by generic services with NO domain validation. That lets a caller create a
//! branchless company, a company/branch with a malformed NPWP, or a department whose parent/branch
//! belongs to another company — corrupting the org dimension every downstream module trusts.
//!
//! Guarded surface:
//!   - **Company**: READ-ONLY over generic CRUD. The only writer is `OnboardingService`
//!     (`POST /companies/onboard`) — a company is always born with a head-office branch.
//!   - **Branch / Department**: READ + **validated create** and **validated re-point** via
//!     `OrgWriteService` (NPWP format, company existence, same-company parent/branch, no cycle).
//!     Generic update/delete/upsert/bulk are intentionally NOT mounted here.

use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::service::org_write_service::{NewBranch, NewDepartment, OrgWriteError, OrgWriteService};
use crate::OrganizationModule;

use super::{
    create_branch_read_routes, create_company_read_routes, create_department_read_routes,
    create_onboarding_routes,
};

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

fn err_response(e: OrgWriteError) -> axum::response::Response {
    let status = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorBody { error: e.code(), message: e.to_string() })).into_response()
}

// ── Branch ───────────────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBranchBody {
    company_id: Uuid,
    code: String,
    name: String,
    #[serde(default)]
    branch_type: Option<String>,
    #[serde(default)]
    is_head_office: bool,
    #[serde(default)]
    npwp: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default)]
    address: Option<String>,
}

#[derive(Debug, Serialize)]
struct IdResponse {
    id: Uuid,
}

async fn create_branch(
    State(svc): State<Arc<OrgWriteService>>,
    Json(b): Json<CreateBranchBody>,
) -> axum::response::Response {
    match svc
        .create_branch(NewBranch {
            company_id: b.company_id,
            code: b.code,
            name: b.name,
            branch_type: b.branch_type,
            is_head_office: b.is_head_office,
            npwp: b.npwp,
            email: b.email,
            phone: b.phone,
            address: b.address,
        })
        .await
    {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err_response(e),
    }
}

// ── Department ────────────────────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDepartmentBody {
    company_id: Uuid,
    code: String,
    name: String,
    #[serde(default)]
    parent_id: Option<Uuid>,
    #[serde(default)]
    branch_id: Option<Uuid>,
    #[serde(default)]
    is_group: bool,
    #[serde(default)]
    manager_id: Option<Uuid>,
}

async fn create_department(
    State(svc): State<Arc<OrgWriteService>>,
    Json(d): Json<CreateDepartmentBody>,
) -> axum::response::Response {
    match svc
        .create_department(NewDepartment {
            company_id: d.company_id,
            code: d.code,
            name: d.name,
            parent_id: d.parent_id,
            branch_id: d.branch_id,
            is_group: d.is_group,
            manager_id: d.manager_id,
        })
        .await
    {
        Ok(id) => (StatusCode::CREATED, Json(IdResponse { id })).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepointDepartmentBody {
    #[serde(default)]
    parent_id: Option<Uuid>,
    #[serde(default)]
    branch_id: Option<Uuid>,
}

async fn repoint_department(
    State(svc): State<Arc<OrgWriteService>>,
    Path(id): Path<Uuid>,
    Json(body): Json<RepointDepartmentBody>,
) -> axum::response::Response {
    match svc.repoint_department(id, body.parent_id, body.branch_id).await {
        Ok(()) => (StatusCode::OK, Json(IdResponse { id })).into_response(),
        Err(e) => err_response(e),
    }
}

fn create_org_write_routes(svc: Arc<OrgWriteService>) -> Router {
    Router::new()
        .route("/branches", post(create_branch))
        .route("/departments", post(create_department))
        .route("/departments/{id}/repoint", post(repoint_department))
        .with_state(svc)
}

/// Mount the organization module with write paths locked to validated services.
/// **Prefer this over `OrganizationModule::routes()` / `create_organization_routes` for any real
/// deployment** — the latter expose unvalidated generic CRUD.
pub fn create_guarded_organization_routes(m: &OrganizationModule) -> Router {
    Router::new()
        // Company: read-only. Sole writer is onboarding (company + head-office branch, atomic).
        .merge(create_company_read_routes(m.company_service.clone()))
        .merge(create_onboarding_routes(m.onboarding_service.clone()))
        // Branch / Department: read + validated writes.
        .merge(create_branch_read_routes(m.branch_service.clone()))
        .merge(create_department_read_routes(m.department_service.clone()))
        .merge(create_org_write_routes(m.org_write_service.clone()))
}
