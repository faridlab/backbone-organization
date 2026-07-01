//! Onboarding REST handler — hand-authored (user-owned; never regenerated).
//!
//! Exposes the one non-CRUD endpoint this module owns:
//!   POST /companies/onboard  → create a Company + its head-office Branch atomically.
//! Everything else is the generated 12-endpoint CRUD per entity.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::service::{OnboardError, OnboardRequest, OnboardingService};

#[derive(Debug, Deserialize)]
pub struct OnboardCompanyBody {
    pub code: String,
    pub legal_name: String,
    #[serde(default)]
    pub trade_name: Option<String>,
    #[serde(default)]
    pub npwp: Option<String>,
    #[serde(default)]
    pub nib: Option<String>,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub base_currency: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub hq_branch_code: Option<String>,
    #[serde(default)]
    pub hq_branch_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OnboardCompanyResponse {
    pub company_id: Uuid,
    pub hq_branch_id: Uuid,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

async fn onboard_company(
    State(service): State<Arc<OnboardingService>>,
    Json(body): Json<OnboardCompanyBody>,
) -> impl IntoResponse {
    let req = OnboardRequest {
        code: body.code,
        legal_name: body.legal_name,
        trade_name: body.trade_name,
        npwp: body.npwp,
        nib: body.nib,
        entity_type: body.entity_type,
        base_currency: body.base_currency,
        email: body.email,
        phone: body.phone,
        hq_branch_code: body.hq_branch_code,
        hq_branch_name: body.hq_branch_name,
    };

    match service.onboard(req).await {
        Ok(result) => (
            StatusCode::CREATED,
            Json(OnboardCompanyResponse {
                company_id: result.company_id,
                hq_branch_id: result.hq_branch_id,
            }),
        )
            .into_response(),
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

/// `POST /companies/onboard` — atomic company + head-office branch creation.
pub fn create_onboarding_routes(service: Arc<OnboardingService>) -> Router {
    Router::new()
        .route("/companies/onboard", post(onboard_company))
        .with_state(service)
}

/// One-call composer: the generated 12-endpoint CRUD per entity (the **unguarded** surface)
/// **plus** this module's onboarding endpoint. For production prefer
/// [`create_guarded_organization_routes`] (company read-only, validated branch/dept writes);
/// this variant is for trusted/admin/seeding contexts that want full CRUD + onboarding.
///
/// Lives in a `user_owned` file (see `metaphor.codegen.yaml`) so it survives regeneration —
/// unlike `routes/mod.rs`, whose `<<< CUSTOM HANDLERS >>>` block the generator rewrites.
pub fn create_organization_routes(module: &crate::OrganizationModule) -> Router {
    module
        .all_crud_routes()
        .merge(create_onboarding_routes(module.onboarding_service.clone()))
}
