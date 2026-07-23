# ADR-003: Tenant isolation boundary — reaffirm and reconcile drift

**Status**: Accepted — **Applied 2026-07-23**
**Deciders**: Farid (owner)
**Related**: ADR-001 (organization boundary — decided schema-per-tenant), ADR-002 (guarded write paths), backbone-pos ADR-0008 (RLS fence pattern), `schema/models/company.model.yaml` header (the drift source)

## Context

A council review (2026-07-23, see `backbone-organization` council transcript) of whether to adopt
`salt-laravel-organization`'s models flagged a blocking precondition: *"settle the tenancy/isolation
boundary via ADR before importing ANY external 'organization' concept."* The council read the
`company.model.yaml` header (lines 11–22), which states:

> THE ISOLATION BOUNDARY IS UNDECIDED… Two earlier headers each asserted an answer and neither was
> earned… State only what is verified until an ADR settles it.

**That comment contradicts an already-Accepted ADR.** ADR-001 (Applied 2026-07-01) Decision #3
settled this explicitly:

> **Schema-per-tenant multi-tenancy.** No `provider_id`/`tenant_id` column. A tenant schema may hold
> several Companies (legal entities); isolation is a DB-layer concern.

The PRD ("Out") and FSD agree. The header was added on **2026-07-15** — two weeks *after* ADR-001
was applied — and re-opened a question the module had already answered. So the real blocker is not a
missing decision; it is **drift between an accepted ADR and a later code comment**, plus a genuine
conflation of two different boundaries that the header runs together.

The two "contradictory candidates" the header names are not actually in conflict once separated:

1. **The tenant isolation fence** — what stops tenant A seeing tenant B's data. Decided in ADR-001:
   the **deployment/schema** is the tenant; no tenant column lives in this module. RLS at the app
   role enforces it (the same pattern backbone-pos codified in its ADR-0008, and the one the recent
   `require_known_company` guard + `org_write_service` RLS scoping commits implement).
2. **The books dimension** — `company_id`. This is **not a tenant**. A principal crosses it routinely
   and legitimately (consolidated reporting, intercompany postings — exactly why `parent_company_id`
   + `is_default` exist). Treating `company_id` as a fence would break consolidation.

The header's "tenant entity above Company" candidate (sapiens `OrganizationUser`/`OrganizationRole`/
`OrganizationPermission`) is an **auth/RBAC** concern owned by sapiens, not an entity this books
module should model. Its absence here is correct, not a gap.

## Decision

1. **Reaffirm ADR-001.** The tenant isolation boundary is **schema-per-tenant at the DB layer**,
   enforced by RLS at the app role. There is no `tenant_id`/`provider_id` column in this module and
   there will not be one. This ADR does not reverse ADR-001; it removes the ambiguity the Jul-15
   header introduced.
2. **`company_id` is a books dimension, NOT a tenant.** It is the legal-entity owner of a document,
   not an isolation fence. Multiple companies may legitimately coexist in one tenant schema, and
   consolidated/intercompany flows cross `company_id` by design. Any code or comment that frames
   `company_id` as a tenant boundary is wrong.
3. **Two layers, stated once, in one place.** This ADR is the single source for both: (a) tenant =
   schema (RLS-enforced), (b) books = `company_id` (a cross-able dimension). No other file may
   re-litigate either question without superseding this ADR.
4. **New entities inherit the existing fence — no new boundary claim.** Adding an entity (e.g.
   `Industry`) does not assert a tenancy position: it lives in the same tenant schema and is
   RLS-scoped like every other table. This is what unblocks the deferred `industry.model.yaml`.
5. **Header correction is mandatory.** The `company.model.yaml` header must be rewritten to point at
   ADR-001 + ADR-003 and stop claiming the boundary is undecided. (Action item, below.)

## Consequences

- The council's blocking precondition is removed. The `industry.model.yaml` sketch (KBLI 2020
  lookup) is safe to land once this ADR is Accepted; its tenancy story is "same schema-per-tenant
  fence as Company," nothing new.
- The module's ubiquitous language is settled: **Organization module = books-owner identity
  (Company/Branch/Department)**, not a CRM directory and not a tenant registry. This is the answer
  to the "should we adopt salt-laravel?" question — no.
- RLS, not application code, remains the isolation mechanism; the ADR-002 guarded write paths and
  the `require_known_company` guard operate *inside* the fence the tenant schema already provides.
- Future regions/overlays extend via `entity_type`/identity overlays, not by introducing a tenant
  entity here.

## Parking lot (deferred)

- **Action — header rewrite:** replace the `company.model.yaml` "ISOLATION BOUNDARY IS UNDECIDED"
  block with a two-line pointer to ADR-001 §3 and ADR-003 §1–2. Do this in the same PR that lands
  this ADR so the contradiction does not survive.
- Cross-tenant consolidated read paths (a read-role that intentionally unions companies for group
  reporting) — out of scope here; will need their own ADR when intercompany/consolidation lands.
- Whether a future CRM/partner-directory module (the rightful home for salt-style demographics)
  references `organization.Company` via `company_id` — yes by default, but owned by that module.
