# backbone-organization — PRD

> Tier-0 master-data module. Owns the **organizational backbone** every other ERP module
> references: **Company** (legal entity / books owner), **Branch** (operational location),
> **Department** (org unit / accounting dimension). Indonesia-first.

## Problem

Every transactional module (accounting, selling, buying, inventory, payroll) needs a stable
answer to "which legal entity owns these books?" and "which location / org unit did this happen
in?". In ERPNext this lives scattered across Company / Branch / Department / Cost Center DocTypes
inside the Setup + Accounts modules, tangled with GL config. We need one small, independent module
that owns these identities and nothing else, so the rest of the suite can reference them by
logical FK without importing a monolith.

## Scope

**In:**
- `Company` — legal entity. Indonesia statutory identity (NPWP, NIB, `entity_type` = PT/CV/…),
  functional currency (default IDR), fiscal-year start month, group/consolidation parent link.
- `Branch` — operational location under a Company (head office / branch / warehouse / outlet /
  factory). May carry its own NPWP for local reporting.
- `Department` — hierarchical org unit under a Company (tree; group nodes; optional branch link;
  logical manager reference to `sapiens.User`).
- **Onboarding**: create a Company and its head-office Branch atomically (the one non-CRUD flow).
- 12 standard CRUD endpoints per entity (generated).

**Out (owned elsewhere / deferred):**
- **Cost Center** — owned by `backbone-accounting` (a controlling dimension; see its GL-core ADR).
- **Chart of Accounts, fiscal periods, GL** — `backbone-accounting`.
- **Users / auth / RBAC** — `backbone-sapiens` (referenced as a logical FK only).
- **Tax rules** (PPN/PPh/e-Faktur) — deferred to `backbone-tax-id` overlay. We store identifiers
  (NPWP/NIB) as neutral fields, not tax behavior.
- Multi-tenant isolation — schema-per-tenant at the DB layer; **no `provider_id`/`tenant_id`
  column** here.

## Personas

- **Tenant admin** — onboards the company, sets up branches/departments during initial config.
- **Consuming modules** — read Company/Branch/Department by logical FK to stamp `company_id`,
  `branch_id`, `department_id` dimensions on their transactions.

## Success criteria

- A company + head-office branch can be created in one atomic call; no half-created company.
- NPWP is validated (15 or 16 digits) and unique per tenant schema; company `code` is unique.
- Zero horizontal Cargo dependencies on other modules (identities referenced by logical FK only).
- Schema is the SSoT; generated CRUD + migrations reproduce with no drift.

## Indonesia-first notes

- `entity_type` enumerates Indonesian legal forms (PT default, CV, Firma, Perorangan, Koperasi,
  Yayasan, BUMN, Other).
- `npwp` (Nomor Pokok Wajib Pajak) and `nib` (Nomor Induk Berusaha) are first-class identity
  fields; base currency defaults to `IDR`, country to `ID`. Tax *mechanics* remain a separate
  overlay — this module only holds the identifiers.
