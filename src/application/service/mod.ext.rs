// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

pub mod onboarding_service;
pub mod org_write_service;
pub mod hierarchy_service;
pub use onboarding_service::{
    validate_npwp, OnboardError, OnboardRequest, OnboardResult, OnboardingService,
};
pub use org_write_service::{NewBranch, NewDepartment, OrgWriteError, OrgWriteService};
pub use hierarchy_service::{
    BranchHierarchy, CompanyHierarchy, CompanyInfo, DepartmentNode, HierarchyError,
    HierarchyService,
};
