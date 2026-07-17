//! Company onboarding — create a Company and its head-office Branch atomically.
//!
//! Hand-authored behavior (user-owned; see `metaphor.codegen.yaml`). Every company needs a
//! default (head-office) branch; doing it in one transaction avoids a half-created company.
//! Also validates the Indonesian NPWP format. Proven by `tests/onboarding_golden_cases.rs`.

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    BranchRepository, CompanyRepository, NewCompanyRow, NewHeadOfficeBranchRow,
};

#[derive(Debug, Clone)]
pub struct OnboardRequest {
    pub code: String,
    pub legal_name: String,
    pub trade_name: Option<String>,
    pub npwp: Option<String>,
    pub nib: Option<String>,
    pub entity_type: Option<String>,   // CompanyEntityType label; default 'pt'
    pub base_currency: Option<String>, // default 'IDR'
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Head-office branch code/name (defaults: "HQ" / "Head Office").
    pub hq_branch_code: Option<String>,
    pub hq_branch_name: Option<String>,
}

impl OnboardRequest {
    pub fn new(code: &str, legal_name: &str) -> Self {
        Self {
            code: code.to_string(),
            legal_name: legal_name.to_string(),
            trade_name: None,
            npwp: None,
            nib: None,
            entity_type: None,
            base_currency: None,
            email: None,
            phone: None,
            hq_branch_code: None,
            hq_branch_name: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnboardResult {
    pub company_id: Uuid,
    pub hq_branch_id: Uuid,
}

#[derive(Debug)]
pub enum OnboardError {
    DuplicateCode(String),
    DuplicateNpwp(String),
    InvalidNpwp(String),
    Db(sqlx::Error),
}

impl OnboardError {
    pub fn code(&self) -> &'static str {
        match self {
            OnboardError::DuplicateCode(_) => "duplicate_company_code",
            OnboardError::DuplicateNpwp(_) => "duplicate_npwp",
            OnboardError::InvalidNpwp(_) => "invalid_npwp",
            OnboardError::Db(_) => "internal_error",
        }
    }
    pub fn http_status(&self) -> u16 {
        match self {
            OnboardError::Db(_) => 500,
            _ => 422,
        }
    }
}
impl std::fmt::Display for OnboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardError::DuplicateCode(c) => write!(f, "duplicate_company_code: {c}"),
            OnboardError::DuplicateNpwp(n) => write!(f, "duplicate_npwp: {n}"),
            OnboardError::InvalidNpwp(n) => write!(f, "invalid_npwp: {n}"),
            OnboardError::Db(e) => write!(f, "db_error: {e}"),
        }
    }
}
impl std::error::Error for OnboardError {}
impl From<sqlx::Error> for OnboardError {
    fn from(e: sqlx::Error) -> Self {
        OnboardError::Db(e)
    }
}

/// Validate an Indonesian NPWP: 15 (legacy) or 16 (NIK-based) digits, ignoring separators.
pub fn validate_npwp(npwp: &str) -> bool {
    let digits = npwp.chars().filter(|c| c.is_ascii_digit()).count();
    digits == 15 || digits == 16
}

#[derive(Clone)]
pub struct OnboardingService {
    db_pool: PgPool,
    companies: Arc<CompanyRepository>,
    branches: Arc<BranchRepository>,
}

impl OnboardingService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            companies: Arc::new(CompanyRepository::new(db_pool.clone())),
            branches: Arc::new(BranchRepository::new(db_pool.clone())),
            db_pool,
        }
    }

    pub async fn onboard(&self, req: OnboardRequest) -> Result<OnboardResult, OnboardError> {
        if let Some(npwp) = &req.npwp {
            if !validate_npwp(npwp) {
                return Err(OnboardError::InvalidNpwp(npwp.clone()));
            }
        }

        // Unique company code (fast pre-check; the partial unique index is the real arbiter).
        let exists: Option<Uuid> = self
            .companies
            .find_live_id_by_code(&self.db_pool, &req.code)
            .await?;
        if exists.is_some() {
            return Err(OnboardError::DuplicateCode(req.code.clone()));
        }

        let mut tx = self.db_pool.begin().await?;

        let company_id = Uuid::new_v4();
        let entity_type = req.entity_type.clone().unwrap_or_else(|| "pt".to_string());
        let base_currency = req.base_currency.clone().unwrap_or_else(|| "IDR".to_string());

        let company_insert = self
            .companies
            .insert_company_on(
                &mut tx,
                &NewCompanyRow {
                    id: company_id,
                    code: &req.code,
                    legal_name: &req.legal_name,
                    trade_name: req.trade_name.as_ref(),
                    npwp: req.npwp.as_ref(),
                    nib: req.nib.as_ref(),
                    entity_type: &entity_type,
                    base_currency: &base_currency,
                    email: req.email.as_ref(),
                    phone: req.phone.as_ref(),
                },
            )
            .await;

        // A unique index rejects us if a concurrent onboard grabbed the code, or if the NPWP
        // is already registered to another company. Distinguish by constraint name.
        if let Err(ref e) = company_insert {
            if let Some(dbe) = e.as_database_error() {
                if dbe.is_unique_violation() {
                    let constraint = dbe.constraint().unwrap_or("");
                    drop(tx);
                    return if constraint.contains("npwp") {
                        Err(OnboardError::DuplicateNpwp(
                            req.npwp.clone().unwrap_or_default(),
                        ))
                    } else {
                        Err(OnboardError::DuplicateCode(req.code.clone()))
                    };
                }
            }
        }
        company_insert?;

        let hq_branch_id = Uuid::new_v4();
        let branch_code = req.hq_branch_code.clone().unwrap_or_else(|| "HQ".to_string());
        let branch_name = req.hq_branch_name.clone().unwrap_or_else(|| "Head Office".to_string());
        self.branches
            .insert_head_office_on(
                &mut tx,
                &NewHeadOfficeBranchRow {
                    id: hq_branch_id,
                    company_id,
                    code: &branch_code,
                    name: &branch_name,
                },
            )
            .await?;

        tx.commit().await?;

        tracing::info!(target: "organization.onboarding", %company_id, %hq_branch_id, code = %req.code, "company onboarded");

        Ok(OnboardResult { company_id, hq_branch_id })
    }
}
