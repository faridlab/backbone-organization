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
//!
//! Every write above is additionally **tenant-guarded**: `company_auth` proves the caller's tenant
//! from a signed Bearer token and the handlers stamp `company_id` from that token, never from the
//! request body — a client must not be able to name the company it writes into.
//!
//! `POST /companies/onboard` is deliberately NOT behind the tenant guard: it *creates* the tenant,
//! so there is no pre-existing `company_id` a token could carry. It is an unauthenticated-by-design
//! signup seam whose access control belongs to the composing service, not to this guard.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use backbone_auth::company::{company_auth, CompanyContext, CompanyVerifier};
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
    // No `company_id`: the tenant is derived from the signed token via `CompanyContext`, never from
    // the request body — a client must not be able to name the company it creates a branch in.
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
    tenant: CompanyContext,
    Json(b): Json<CreateBranchBody>,
) -> axum::response::Response {
    match svc
        .create_branch(NewBranch {
            company_id: tenant.company_id,
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
    // No `company_id`: the tenant comes from the signed token (`CompanyContext`), not the body.
    code: String,
    name: String,
    #[serde(default)]
    parent_id: Option<Uuid>,
    // `branch_id` stays in the body: unlike the tenant, it is a *domain choice* (which of the
    // tenant's branches this department sits under), and `OrgWriteService` already validates that it
    // belongs to `company_id` — i.e. to the token's tenant.
    #[serde(default)]
    branch_id: Option<Uuid>,
    #[serde(default)]
    is_group: bool,
    #[serde(default)]
    manager_id: Option<Uuid>,
}

async fn create_department(
    State(svc): State<Arc<OrgWriteService>>,
    tenant: CompanyContext,
    Json(d): Json<CreateDepartmentBody>,
) -> axum::response::Response {
    match svc
        .create_department(NewDepartment {
            company_id: tenant.company_id,
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

fn create_org_write_routes(svc: Arc<OrgWriteService>, verifier: CompanyVerifier) -> Router {
    Router::new()
        .route("/branches", post(create_branch))
        .route("/departments", post(create_department))
        .route("/departments/{id}/repoint", post(repoint_department))
        // Every write above is tenant-scoped: `company_auth` rejects a request whose token is absent,
        // invalid, or carries no `company_id`, so a handler only ever runs with a proven tenant.
        //
        // `route_layer`, not `layer`: `layer` would also wrap this router's fallback, so once merged
        // every *unmatched* path (e.g. the generic CRUD paths this surface deliberately does not
        // mount) would answer 401 instead of 404 — leaking "auth required" for routes that do not
        // exist, and masking the CRUD-bypass probes.
        .route_layer(from_fn_with_state(verifier, company_auth))
        .with_state(svc)
}

/// Mount the organization module with write paths locked to validated services.
/// **Prefer this over `OrganizationModule::routes()` / `create_organization_routes` for any real
/// deployment** — the latter expose unvalidated generic CRUD.
///
/// The composing service builds one [`CompanyVerifier`] from its JWT secret and passes it here; the
/// branch/department write surface derives `company_id` from the token, so no tenant crosses the
/// wire in a body. `POST /companies/onboard` is exempt — it creates the tenant itself.
pub fn create_guarded_organization_routes(
    m: &OrganizationModule,
    verifier: CompanyVerifier,
) -> Router {
    Router::new()
        // Company: read-only. Sole writer is onboarding (company + head-office branch, atomic).
        .merge(create_company_read_routes(m.company_service.clone()))
        .merge(create_onboarding_routes(m.onboarding_service.clone()))
        // Branch / Department: read + validated writes.
        .merge(create_branch_read_routes(m.branch_service.clone()))
        .merge(create_department_read_routes(m.department_service.clone()))
        .merge(create_org_write_routes(m.org_write_service.clone(), verifier))
}
