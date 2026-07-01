# Organization Workflows

Declarative specs of the module's multi-step sagas. The hand-authored Rust is the executable
truth; these YAML files document the intended orchestration and are the readable companion to the
golden cases.

| Workflow | Saga | Implemented in | Proven by |
|----------|------|----------------|-----------|
| `company-onboarding.workflow.yaml` | Company + head-office Branch, atomic | `src/application/service/onboarding_service.rs` | `tests/onboarding_golden_cases.rs` |

Status lifecycles (Company `active→inactive→suspended→dissolved`; Branch/Department `active↔inactive`)
are declared as state machines in `schema/hooks/organization.hook.yaml`, not as workflows — they are
single-entity transitions, not multi-step sagas.
