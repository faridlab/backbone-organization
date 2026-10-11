// This directory's extension (hand-written; ADR-0031). The generated mod.rs (lib.rs for
// src/) pulls it in with include!, so its lines resolve against this directory.

pub mod onboarding_handler;
pub mod guarded_routes;
pub mod hierarchy_handler;
pub use onboarding_handler::{create_onboarding_routes, create_organization_routes};
pub use guarded_routes::create_guarded_organization_routes;
pub use hierarchy_handler::create_hierarchy_routes;
