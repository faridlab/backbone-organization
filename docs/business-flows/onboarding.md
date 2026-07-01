# Business Flow — Company Onboarding

> Owning module: `backbone-organization` · Implemented in
> `src/application/service/onboarding_service.rs`, proven by `tests/onboarding_golden_cases.rs`.
> Workflow spec: `schema/workflows/company-onboarding.workflow.yaml` · Rules: R1–R4 in
> `schema/hooks/organization.hook.yaml`.

The single flow that brings a new legal entity onto the platform. A company is **never** created
without a head-office branch, so downstream modules can always resolve a `branch_id` dimension.

## Actors
- **Tenant admin** — initiates onboarding during initial setup.

## Preconditions
- The tenant's Postgres schema (`organization`) exists and is migrated.
- The company `code` is not already used by a non-deleted company; the NPWP (if given) is not
  already registered.

## Main path
1. Admin calls `POST /companies/onboard` with at least `{ code, legalName }` (optionally `npwp`,
   `nib`, `entityType`, `baseCurrency`, contact fields, `hqBranchCode`/`hqBranchName`).
2. **Validate NPWP** (R1): if present, must be 15 or 16 digits (separators ignored). Else → 422
   `invalid_npwp`, nothing written.
3. **Pre-check code uniqueness** (R3): fast SELECT; the partial unique index is the real arbiter.
4. **In one transaction**: insert the Company (`status=active`, `base_currency` default `IDR`,
   `entity_type` default `pt`), then insert the head-office Branch
   (`branch_type=head_office`, `is_head_office=true`, code `HQ` / name `Head Office` by default).
5. **Commit**; respond `201 { companyId, hqBranchId }` and (intended) emit `CompanyOnboarded`.

## Business rules
- **R1** NPWP format (15/16 digits) → `invalid_npwp`.
- **R2** Company is born with a head-office branch (structural — no generic company create route).
- **R3** Unique company code → `duplicate_company_code`.
- **R4** Unique company NPWP → `duplicate_npwp` (distinguished from R3 by the violated constraint).

## Alternate / failure paths
- Invalid NPWP → 422 `invalid_npwp`, no rows written.
- Duplicate code → 422 `duplicate_company_code`.
- Duplicate NPWP (different code) → 422 `duplicate_npwp`.
- Two concurrent onboards of the same code → exactly one wins (partial unique index); the loser
  gets `duplicate_company_code`.

## Postconditions
- Exactly one Company row and exactly one head-office Branch row exist for the new entity, or
  nothing was written.

See exact numbers in [golden-cases.md](golden-cases.md).
