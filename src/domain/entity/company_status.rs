use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "company_status", rename_all = "snake_case")]
pub enum CompanyStatus {
    Active,
    Inactive,
    Suspended,
    Dissolved,
}

impl std::fmt::Display for CompanyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Inactive => write!(f, "inactive"),
            Self::Suspended => write!(f, "suspended"),
            Self::Dissolved => write!(f, "dissolved"),
        }
    }
}

impl FromStr for CompanyStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "suspended" => Ok(Self::Suspended),
            "dissolved" => Ok(Self::Dissolved),
            _ => Err(format!("Unknown CompanyStatus variant: {}", s)),
        }
    }
}

impl Default for CompanyStatus {
    fn default() -> Self {
        Self::Active
    }
}
