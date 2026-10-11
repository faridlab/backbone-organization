//! The module's extension (hand-written; ADR-0031): what the module adds beside its
//! generated services. The generated module and builder carry `ModuleExt` and
//! `ModuleBuilderExt` and dereference to them, so their fields read as `module.field`.

#[allow(unused_imports)]
use super::*;

/// State the module adds; read through the generated module's `Deref`.
pub struct ModuleExt {
    /// Atomic company onboarding (Company + head-office Branch in one transaction).
    pub onboarding_service: Arc<OnboardingService>,
    /// Validated Branch/Department writes (NPWP format, same-company parent/branch, no cycle).
    pub org_write_service: Arc<OrgWriteService>,
    /// Company operational hierarchy read (Company → Branches → Departments tree).
    pub hierarchy_service: Arc<HierarchyService>,
}

/// State the builder adds.
pub struct ModuleBuilderExt {
}

impl Default for ModuleBuilderExt {
    fn default() -> Self {
        Self {
        }
    }
}

impl ModuleBuilderExt {
    /// Build the extension's state from what the generated build made.
    #[allow(unused_variables, clippy::redundant_clone)]
    pub(crate) fn build(self, parts: &ModuleParts<'_>) -> anyhow::Result<ModuleExt> {
        let db_pool = parts.db_pool.clone();
        let onboarding_service = Arc::new(OnboardingService::new(db_pool.clone()));
        let org_write_service = Arc::new(OrgWriteService::new(db_pool.clone()));
        let hierarchy_service = Arc::new(HierarchyService::new(db_pool.clone()));
        Ok(ModuleExt {
            onboarding_service,
            org_write_service,
            hierarchy_service,
        })
    }
}
