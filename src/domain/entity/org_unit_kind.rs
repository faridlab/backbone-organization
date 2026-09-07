use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "org_unit_kind", rename_all = "snake_case")]
pub enum OrgUnitKind {
    Root,
    Company,
    Branch,
}

impl std::fmt::Display for OrgUnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(f, "root"),
            Self::Company => write!(f, "company"),
            Self::Branch => write!(f, "branch"),
        }
    }
}

impl FromStr for OrgUnitKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "root" => Ok(Self::Root),
            "company" => Ok(Self::Company),
            "branch" => Ok(Self::Branch),
            _ => Err(format!("Unknown OrgUnitKind variant: {}", s)),
        }
    }
}

impl Default for OrgUnitKind {
    fn default() -> Self {
        Self::Company
    }
}
