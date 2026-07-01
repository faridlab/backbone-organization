use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "company_entity_type", rename_all = "snake_case")]
pub enum CompanyEntityType {
    Pt,
    Cv,
    Firma,
    Perorangan,
    Koperasi,
    Yayasan,
    Bumn,
    Other,
}

impl std::fmt::Display for CompanyEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pt => write!(f, "pt"),
            Self::Cv => write!(f, "cv"),
            Self::Firma => write!(f, "firma"),
            Self::Perorangan => write!(f, "perorangan"),
            Self::Koperasi => write!(f, "koperasi"),
            Self::Yayasan => write!(f, "yayasan"),
            Self::Bumn => write!(f, "bumn"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl FromStr for CompanyEntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pt" => Ok(Self::Pt),
            "cv" => Ok(Self::Cv),
            "firma" => Ok(Self::Firma),
            "perorangan" => Ok(Self::Perorangan),
            "koperasi" => Ok(Self::Koperasi),
            "yayasan" => Ok(Self::Yayasan),
            "bumn" => Ok(Self::Bumn),
            "other" => Ok(Self::Other),
            _ => Err(format!("Unknown CompanyEntityType variant: {}", s)),
        }
    }
}

impl Default for CompanyEntityType {
    fn default() -> Self {
        Self::Pt
    }
}
