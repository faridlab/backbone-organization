# Organization — Golden Cases (the oracle)

Exact expected results for the module's flows. These mirror the executable tests one-to-one.

## Onboarding (`tests/onboarding_golden_cases.rs`)

| Case | Input | Expected |
|------|-------|----------|
| **OGC-1** | onboard `{code, legalName, npwp(15 digits), entityType:pt}` | `201`; 1 Company (status `active`, `base_currency` IDR, `entity_type` pt), exactly 1 Branch (`head_office`, `is_head_office=true`, code `HQ`); returned `hqBranchId` == that branch. |
| **OGC-2** | onboard with `npwp:"12345"` | `422 invalid_npwp`; **0** companies written. |
| **OGC-3** | onboard a code that already exists | `422 duplicate_company_code`. |
| **OGC-3b** | onboard a new code reusing an existing NPWP | `422 duplicate_npwp`. |
| **OGC-4** | two concurrent onboards of the same code | exactly **1** succeeds; DB holds exactly 1 company for the code. |
| **OGC-5** (unit) | `validate_npwp` | accepts 15 & 16 digits; rejects 5, 18, empty. |

## Guarded write path (`tests/integrity_probes.rs`)

| Case | Input via guarded routes | Expected |
|------|--------------------------|----------|
| **IGC-1** | `POST /companies` (generic create) | not routed → `405/404` (company writes only via onboarding). |
| **IGC-2** | `POST /branches` with `npwp:"12345"` | `422` (invalid NPWP rejected). |
| **IGC-3** | `POST /departments` with a cross-company `parentId` | `422` (cross-company parent rejected). |
| **IGC-4** | valid branch (15-digit NPWP) and valid same-company department | both `201`. |

## Sign / default conventions
- New Company: `status=active`, `base_currency=IDR`, `entity_type=pt`, `country=ID` unless overridden.
- New head-office Branch: `branch_type=head_office`, `is_head_office=true`, `status=active`.
- Soft-delete via `metadata->>'deleted_at'`; all uniqueness is partial on not-deleted rows.
