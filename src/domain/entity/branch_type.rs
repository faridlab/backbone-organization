use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "branch_type", rename_all = "snake_case")]
pub enum BranchType {
    HeadOffice,
    Branch,
    Warehouse,
    Outlet,
    Factory,
}

impl std::fmt::Display for BranchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeadOffice => write!(f, "head_office"),
            Self::Branch => write!(f, "branch"),
            Self::Warehouse => write!(f, "warehouse"),
            Self::Outlet => write!(f, "outlet"),
            Self::Factory => write!(f, "factory"),
        }
    }
}

impl FromStr for BranchType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "head_office" => Ok(Self::HeadOffice),
            "branch" => Ok(Self::Branch),
            "warehouse" => Ok(Self::Warehouse),
            "outlet" => Ok(Self::Outlet),
            "factory" => Ok(Self::Factory),
            _ => Err(format!("Unknown BranchType variant: {}", s)),
        }
    }
}

impl Default for BranchType {
    fn default() -> Self {
        Self::Branch
    }
}
