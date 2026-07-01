# backbone-organization — FSD

Functional spec. Schema (`schema/models/*.model.yaml`) is the SSoT; this documents behavior and
integration points that the schema does not encode.

## Entities

| Entity | Table | Key identity | Notes |
|--------|-------|--------------|-------|
| Company | `companies` | `code` (unique), `npwp` (unique when present) | Books owner. `entity_type`, `base_currency` (IDR), `fiscal_year_start_month`, self-FK `parent_company_id` for consolidation. |
| Branch | `branches` | `(company_id, code)` unique | `branch_type`, `is_head_office`, optional own `npwp`. |
| Department | `departments` | `(company_id, code)` unique | Tree via `parent_id`, `is_group`, optional `branch_id`, logical `manager_id` → `sapiens.User`. |

Soft-delete is via `metadata` JSONB (`metadata->>'deleted_at'`); unique indexes are partial on
`deleted_at IS NULL`.

## Endpoints

- **Generated CRUD** — 12 Backbone endpoints per entity (list / create / get / update / patch /
  soft_delete / restore / empty_trash / bulk_create / upsert / find_by_id / list_deleted), mounted
  by `OrganizationModule::routes()`.
- **Non-CRUD (hand-authored):**
  - `POST /companies/onboard` → create Company + head-office Branch atomically.
    Body: `{ code, legal_name, trade_name?, npwp?, nib?, entity_type?, base_currency?, email?,
    phone?, hq_branch_code?="HQ", hq_branch_name?="Head Office" }`.
    `201 { company_id, hq_branch_id }`.
    Errors: `422 invalid_npwp`, `422 duplicate_company_code`, `422 duplicate_npwp`, `500 internal_error`.

## Onboarding flow (the one saga)

1. Validate NPWP if provided (15 or 16 digits, separators ignored). Reject → `invalid_npwp`, no write.
2. Pre-check company `code` uniqueness (fast path); the partial unique index is the real arbiter.
3. In one transaction: `INSERT` company (status `active`, currency default `IDR`, entity_type
   default `pt`) → `INSERT` head-office branch (`branch_type=head_office`, `is_head_office=true`).
4. On unique violation during insert: roll back, and by constraint name return `duplicate_npwp`
   or `duplicate_company_code`. Concurrent onboards of the same code → exactly one winner.
5. Commit. Return both ids.

Rationale: every company needs a default location to which transactions can attach; creating it
atomically avoids a company with no branch. Proven by `tests/onboarding_golden_cases.rs`.

## Integration points (logical FKs — no DB FK, no Cargo edge)

- `Department.manager_id` → `sapiens.User.id` (`@exclude_from_foreign_key_check`).
- Downstream modules reference `Company.id` (as `company_id`), `Branch.id` (`branch_id`),
  `Department.id` (`department_id`) as accounting/org dimensions. `backbone-accounting` schema
  comments these as `logical FK to organization.Company/Branch/Department`.

## Behavior specs (declarative)

- **Hooks** — `schema/hooks/organization.hook.yaml`: status state machines (Company
  active→inactive→suspended→dissolved; Branch/Department active↔inactive) + write-path rules R1–R9
  + domain events. `index.hook.yaml` is the module hook index.
- **Workflow** — `schema/workflows/company-onboarding.workflow.yaml`: the Company + head-office
  Branch onboarding saga.
- **Business flows + oracle** — `docs/business-flows/` (onboarding, org-structure, golden-cases)
  and the BDD scenarios in `tests/features/onboarding.feature`. The executable oracle is
  `tests/onboarding_golden_cases.rs` + `tests/integrity_probes.rs`.

## Non-goals in code

No cost centers, no GL, no tax computation, no user management. See [prd.md](prd.md) "Out".
