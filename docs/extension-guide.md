# backbone-organization — Extension Guide

How a consuming service composes and extends this module without editing generated code. Follows
the workspace Extension Contract (`docs/erp/extension-contract.md`).

## Composing into a service

```rust
use backbone_auth::tenant::TenantVerifier;
use backbone_organization::{OrganizationModule, create_guarded_organization_routes};

let organization = OrganizationModule::builder()
    .with_database(pool.clone())
    .build()?;

// RECOMMENDED for any real deployment: guarded composition.
// - Company: read-only; the only writer is onboarding (POST /companies/onboard).
// - Branch/Department: read + validated create/repoint (NPWP format, same-company parent/branch),
//   behind a tenant guard: `company_id` is taken from the signed Bearer token, never the body.
let verifier = TenantVerifier::hs256(jwt_secret.as_bytes());
let app = axum::Router::new().merge(create_guarded_organization_routes(&organization, verifier));
```

Three mounts exist, from safest to widest:

| Function | Company writes | Branch/Dept writes | Use for |
|----------|----------------|--------------------|---------|
| `create_guarded_organization_routes` | onboarding only | validated create/repoint | **default / production** |
| `create_organization_routes` | open generic CRUD | open generic CRUD | trusted/admin + onboarding endpoint |
| `OrganizationModule::routes()` | open generic CRUD | open generic CRUD | generated CRUD only (no onboarding) |

The wider two expose the unvalidated generated CRUD — a well-formed request can create a branchless
company, a bad-NPWP record, or a cross-company department. Use them only in trusted contexts
(seeding, admin tooling behind auth). See [ADR-002](adr/ADR-002-guarded-write-paths.md).

`organization.onboarding_service` and `organization.org_write_service` are also exposed if you want
to call the validated operations directly from your own handler or a seed step.

## Public / stable surface

- **Entities & DTOs** — `Company`/`Branch`/`Department` and their generated `Create*`/`Update*`/
  `*Response` DTOs.
- **Onboarding API** — `OnboardRequest`, `OnboardResult`, `OnboardError`, `OnboardingService`,
  `validate_npwp`, and the routes `create_onboarding_routes` / `create_organization_routes`.
- **Logical FK identities** — `Company.id`, `Branch.id`, `Department.id`. Reference these from
  other contexts as `company_id` / `branch_id` / `department_id` dimensions. **Never** add a DB
  foreign key across the module boundary; keep it a logical reference + ACL translation.

## Supported-but-coupled

- A sibling `*_custom.rs` service in your own module can call `OnboardingService`, or add its own
  behavior over these entities. It survives regeneration.

## Internal (not a contract)

- `// <<< CUSTOM ... // END CUSTOM` marker blocks inside generated aggregators (`lib.rs`,
  `service/mod.rs`, `presentation/http/mod.rs`) preserve *this module's own* wiring only. Do not
  rely on them as a cross-module extension seam.
- `routes/mod.rs` is fully generated — its `<<< CUSTOM HANDLERS >>>` block is **rewritten** on
  regen. Hand-authored routes live in `presentation/http/onboarding_handler.rs` (a `user_owned`
  file) instead.

## Regeneration safety

Hand-authored files are listed in `metaphor.codegen.yaml` `user_owned:` and are never touched by
`metaphor schema schema generate --force`:

- `src/application/service/onboarding_service.rs`
- `src/presentation/http/onboarding_handler.rs`
- `tests/onboarding_golden_cases.rs`
- `docs/**`

Verified: a `--force` regen preserves all CUSTOM markers and leaves the user_owned files intact;
`cargo check` and the golden cases stay green.
