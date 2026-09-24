//! Guarded route composition — the RECOMMENDED way to mount the organization module.
//!
//! Hand-authored (user-owned; see `metaphor.codegen.yaml`). Closes the CRUD-bypass the council
//! flagged: the generated `routes()` exposes full mutable CRUD (POST/PATCH/upsert/bulk) on every
//! entity, backed by generic services with NO domain validation. That lets a caller create a
//! malformed NPWP, or a department whose parent/branch points at nothing — corrupting the org
//! dimension every downstream module trusts.
//!
//! Guarded surface:
//!   - **Company**: READ-ONLY over generic CRUD. The only writer is `OnboardingService`
//!     (`POST /companies/onboard`) — a company is always born with its org-units node and a
//!     head-office branch.
//!   - **Branch / Department**: READ + **validated create** and **validated re-point** via
//!     `OrgWriteService` (NPWP format, node-kind/visibility checks, no self-parent).
//!     Generic update/delete/upsert/bulk are intentionally NOT mounted here.
//!
//! Every write above is additionally **org-guarded** (ADR-0029, the org-composed shape): the
//! module ships bare of authentication and the composing service wraps this router in its org
//! scope middleware (`org_auth` + the tenant router). A new branch attaches under the caller's
//! ACTING NODE — the signed `acting_unit_id`, never a request-body field — and the write
//! service validates that node's kind (company or branch) before anything is written. A
//! department's links are validated by VISIBILITY (a link outside the caller's subtree reads as
//! absent), so no tenant axis crosses the wire at all.
//!
//! `POST /companies/onboard` is deliberately NOT behind the org guard: it *creates* the tenant,
//! so there is no pre-existing node a token could carry. It is an unauthenticated-by-design
//! signup seam whose access control belongs to the composing service, not to this guard.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use backbone_auth::org::OrgContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::service::org_write_service::{NewBranch, NewDepartment, OrgWriteError, OrgWriteService};
use crate::OrganizationModule;

use super::{
    create_branch_read_routes, create_company_industry_read_routes, create_company_read_routes,
    create_department_read_routes, create_hierarchy_routes, create_industry_read_routes,
    create_level_read_routes, create_level_write_routes, create_onboarding_routes,
    create_org_unit_read_routes, create_position_read_routes, create_position_write_routes,
    create_structure_read_routes, create_structure_write_routes,
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
    // No parent field: the attach point is the caller's acting node, derived from the signed
    // token via `OrgContext` — never from the request body.
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
    org: OrgContext,
    Json(b): Json<CreateBranchBody>,
) -> axum::response::Response {
    // The attach point is the caller's ACTING NODE (the signed `acting_unit_id`; the spine
    // preserves company ids as node ids, so a session acting at the company anchors there, and a
    // session acting at a branch may nest beneath it — the write service validates the node's
    // kind either way). Never from the body — a client must not be able to place its branch
    // under someone else's subtree.
    match svc
        .create_branch(NewBranch {
            parent_unit: org.acting_unit_id,
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
    // No tenant field at all: a department carries no tenant axis (ADR-0029). Its links are
    // validated by visibility — a parent or branch outside the caller's subtree reads as absent.
    code: String,
    name: String,
    #[serde(default)]
    parent_id: Option<Uuid>,
    // `branch_id` stays in the body: it is a *domain choice* (which visible branch this
    // department sits under), and `OrgWriteService` validates it by visibility.
    #[serde(default)]
    branch_id: Option<Uuid>,
    #[serde(default)]
    is_group: bool,
    #[serde(default)]
    manager_id: Option<Uuid>,
}

async fn create_department(
    State(svc): State<Arc<OrgWriteService>>,
    _org: OrgContext,
    Json(d): Json<CreateDepartmentBody>,
) -> axum::response::Response {
    // No tenant axis anywhere: `OrgWriteService` validates the links by VISIBILITY (a link
    // outside the caller's subtree reads as absent, ADR-0029), and the composing service's org
    // scope middleware has already bound the request scope the decorator needs. The principal
    // extractor is still demanded — a department write is a WRITE, and no write handler runs
    // without a proven principal even though the row itself carries no unit id.
    match svc
        .create_department(NewDepartment {
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
    _org: OrgContext,
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
        // Bare of auth by design: the composing service's org scope middleware wraps this router,
        // so a handler only ever runs with a proven principal AND a bound org scope. An unknown
        // acting unit never reaches a handler — the org guard resolves the unit against the
        // tenant's own tree and refuses it there, which is the fail-fast "unknown tenant" signal
        // this seam used to carry itself.
        .with_state(svc)
}

/// Mount the organization module with write paths locked to validated services.
/// **Prefer this over `OrganizationModule::routes()` / `create_organization_routes` for any real
/// deployment** — the latter expose unvalidated generic CRUD.
///
/// The router ships bare of authentication (ADR-0029, the org-composed shape): the composing
/// service wraps it in its org scope middleware. A branch write derives its attach point (the
/// caller's acting node) from the signed token, and department writes carry no tenant axis at
/// all — no tenant crosses the wire in a body. `POST /companies/onboard` is exempt — it creates
/// the tenant itself.
pub fn create_guarded_organization_routes(m: &OrganizationModule) -> Router {
    Router::new()
        // Company: read-only. Sole writer is onboarding (company + head-office branch, atomic).
        .merge(create_company_read_routes(m.company_service.clone()))
        .merge(create_onboarding_routes(m.onboarding_service.clone()))
        .merge(create_hierarchy_routes(m.hierarchy_service.clone()))
        // Branch / Department: read + validated writes.
        .merge(create_branch_read_routes(m.branch_service.clone()))
        .merge(create_department_read_routes(m.department_service.clone()))
        .merge(create_org_write_routes(m.org_write_service.clone()))
        // The reference masters the admin surface navigates: industries and
        // the company-industry links (the company screen's picker), the
        // level/position/structure masters, and the org-unit spine rows the
        // entity-shaped views expect. READS ONLY — writes for these ride
        // validated verbs (or later gates), never generic CRUD.
        .merge(create_company_industry_read_routes(m.company_industry_service.clone()))
        .merge(create_industry_read_routes(m.industry_service.clone()))
        // The org masters HR administers: levels, positions, structures.
        // Reference data with generated state machines of their own — the
        // generic write surface (create/update + the state verbs it mounts)
        // is the validated lane; there is no coupling to guard beyond it.
        .merge(create_level_write_routes(m.level_service.clone()))
        .merge(create_position_write_routes(m.position_service.clone()))
        .merge(create_structure_write_routes(m.structure_service.clone()))
        .merge(create_org_unit_read_routes(m.org_unit_service.clone()))
}
