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
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use backbone_auth::company::{company_auth, CompanyContext, CompanyVerifier};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::service::org_write_service::{NewBranch, NewDepartment, OrgWriteError, OrgWriteService};
use crate::OrganizationModule;

use super::{
    create_branch_read_routes, create_company_read_routes, create_department_read_routes,
    create_hierarchy_routes, create_onboarding_routes,
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

/// Tenant-existence guard: reject (401) when the caller's proven `company_id` is not a real
/// `organization.companies` row.
///
/// RLS already fences a non-existent company to empty reads (no cross-tenant leak) — this guard
/// adds an explicit 401 so the caller learns *why* (unknown tenant) instead of seeing empty data,
/// and so a write surface can fail fast before any handler runs.
///
/// **Must be mounted INNER to [`company_auth`]** (i.e. `company_auth` is the outer layer, applied
/// last via `.route_layer`): [`company_auth`] is what inserts the [`CompanyContext`] this reads and
/// — when the app supplies its pool — what binds the request company scope via
/// [`backbone_orm::company_scope::with_request_scope`]. The existence query uses
/// [`backbone_orm::company_scope::fetch_optional_scalar_scoped`], which prefers the request's
/// scoped connection; under that scope `EXISTS(... id = $1)` is RLS-fenced to the caller's own
/// company, so it resolves to `true` iff that company row exists. Outside the request scope (no
/// pool / pre-scope) it fails closed.
pub async fn require_known_company(State(pool): State<PgPool>, req: Request, next: Next) -> Response {
    let Some(ctx) = req.extensions().get::<CompanyContext>().cloned() else {
        return unknown_company("no company principal — mount inner to company_auth");
    };
    let exists = backbone_orm::company_scope::fetch_optional_scalar_scoped(
        &pool,
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM organization.companies WHERE id = $1)",
        )
        .bind(ctx.company_id),
    )
    .await;
    match exists {
        Ok(Some(true)) => next.run(req).await,
        _ => unknown_company("token company_id is not a registered company"),
    }
}

fn unknown_company(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "unauthorized", "message": message })),
    )
        .into_response()
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
        .merge(create_hierarchy_routes(m.hierarchy_service.clone()))
        // Branch / Department: read + validated writes.
        .merge(create_branch_read_routes(m.branch_service.clone()))
        .merge(create_department_read_routes(m.department_service.clone()))
        .merge(create_org_write_routes(m.org_write_service.clone(), verifier))
}

/// Like [`create_guarded_organization_routes`] but additionally requires the caller's token
/// `company_id` to resolve to a real `organization.companies` row (else 401).
///
/// The existence check runs inner to `company_auth` on the validated write surface, so it sees the
/// request company scope `company_auth` binds. RLS already fences an unknown company to empty
/// reads — this adds an explicit, fail-fast 401. Pass the same pool the app registers as a
/// `PgPool` extension (the one `company_auth` upgrades to a request-dedicated scope).
pub fn create_guarded_organization_routes_checked(
    m: &OrganizationModule,
    verifier: CompanyVerifier,
    pool: PgPool,
) -> Router {
    let writes = Router::new()
        .route("/branches", post(create_branch))
        .route("/departments", post(create_department))
        .route("/departments/{id}/repoint", post(repoint_department))
        // `company_auth` (outer, applied last) binds the scope + inserts CompanyContext;
        // `require_known_company` (inner) then reads it and checks existence under that scope.
        .route_layer(from_fn_with_state(pool.clone(), require_known_company))
        .route_layer(from_fn_with_state(verifier, company_auth))
        .with_state(m.org_write_service.clone());
    Router::new()
        .merge(create_company_read_routes(m.company_service.clone()))
        .merge(create_onboarding_routes(m.onboarding_service.clone()))
        .merge(create_hierarchy_routes(m.hierarchy_service.clone()))
        .merge(create_branch_read_routes(m.branch_service.clone()))
        .merge(create_department_read_routes(m.department_service.clone()))
        .merge(writes)
}
