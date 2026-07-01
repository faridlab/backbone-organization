# ADR-001: The organizational-backbone bounded context

**Status**: Accepted — **Applied 2026-07-01**
**Deciders**: Farid (owner)
**Related**: workspace `docs/erp/shared-masters-ownership.md`, backbone-accounting ADR-001 (GL-core boundary)

## Context

Decomposing ERPNext into independent Indonesia-first modules, Tier-0 needs the organizational
master data every transactional module references: **Company** (legal entity / books owner),
**Branch** (operational location), **Department** (org unit). In ERPNext these live across the
Setup and Accounts modules, coupled to GL config (Cost Center, fiscal periods, Chart of Accounts).

Two boundary questions had to be resolved:

1. **Naming collision.** A `backbone-corporate` module already exists in the workspace but models a
   **B2B-sales "corporate account"** domain — not the legal-entity org backbone. Reusing the name
   would overload two unrelated concepts.
2. **What belongs here vs. accounting.** Cost Center is organizational-looking but is fundamentally
   an *accounting/controlling dimension*, and ERPNext ties it to the GL.

## Decision

1. **New module `backbone-organization`** owns Company / Branch / Department. The existing
   `backbone-corporate` is left untouched as the B2B-sales domain. Downstream logical-FK references
   are phrased `organization.Company` / `organization.Branch` / `organization.Department`;
   `backbone-accounting` schema comments were updated `corporate.* → organization.*` accordingly.
2. **Cost Center stays in `backbone-accounting`**, not here. It is a controlling dimension owned by
   the ledger of record (see accounting ADR-001). This module owns only Company/Branch/Department.
3. **Schema-per-tenant multi-tenancy.** No `provider_id`/`tenant_id` column. A tenant schema may
   hold several Companies (legal entities); isolation is a DB-layer concern.
4. **Indonesia-first identity, region-neutral behavior.** NPWP, NIB, and `entity_type` (PT/CV/…)
   are first-class fields; base currency defaults to IDR, country to ID. Tax *mechanics* are
   deferred to the `backbone-tax-id` overlay — this module stores identifiers only.
5. **Cross-module references are logical FKs only** (e.g. `Department.manager_id → sapiens.User`),
   never DB foreign keys or Cargo dependencies. The module keeps zero horizontal edges.

## Consequences

- Clean separation: identities live here; GL config lives in accounting; users live in sapiens.
- The one hand-authored behavior is **atomic onboarding** (Company + head-office Branch in one
  transaction); everything else is generated CRUD. Kept deliberately thin — this is Tier-0 master
  data, not a workflow-heavy domain.
- Consuming modules reference `company_id` / `branch_id` / `department_id` as dimensions and
  translate at their ACL boundary; regenerating any module leaves those logical references intact.
- If a future region needs different legal forms, `entity_type` extends via the overlay pattern
  rather than by branching this module.
