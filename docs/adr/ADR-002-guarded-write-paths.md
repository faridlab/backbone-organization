# ADR-002: Guarded write paths for org master data

**Status**: Accepted — **Applied 2026-07-01**
**Deciders**: Farid (owner), council (module:backbone-organization, focus=maturity, 2026-07-01)
**Related**: ADR-001 (organization boundary); backbone-accounting ADR-002 (ledger write-path integrity — same finding class)

## Context

A maturity-focus council review found the module's core invariants were enforced on exactly **one**
write path (`OnboardingService`) while the generated 12-endpoint CRUD sat next to it, wide open.

`OrganizationModule::routes()` unconditionally merged `create_{company,branch,department}_routes`
— the full `BackboneCrudHandler` (POST/PATCH/PUT-upsert/bulk) backed by generic services with **no
domain validation**. The `validation` cargo feature is off by default, so the generated `#[validate]`
attributes are inert; the only gate was serde required-field deserialization. An in-process probe
(tower oneshot against the real router) confirmed the hole:

- `POST /companies` with a complete camelCase body → **201 Created**, a company with **zero
  branches** (violates "every company has a head-office branch").
- `POST /companies` with `npwp:"12345"` (format-invalid) → **201 Created** (bad tax ID persisted).
- `POST /departments` with a `parent_id` owned by a **different company** → would persist, corrupting
  the org dimension every downstream module rolls up by.

The golden suite was structurally blind to this: it only ever constructed services directly and
never hit the routes — identical to the backbone-accounting suite before its ADR-002.

## Decision

Introduce a **guarded route composition** as the recommended mount, mirroring accounting's
`create_guarded_accounting_routes`:

`create_guarded_organization_routes(&OrganizationModule) -> Router`:
- **Company** — READ-ONLY over generic CRUD. The only writer is `OnboardingService`
  (`POST /companies/onboard`); a company is always born with a head-office branch.
- **Branch / Department** — READ + **validated writes** via a new hand-authored `OrgWriteService`:
  - Branch create validates NPWP format (if present) and that `company_id` exists.
  - Department create validates that `parent_id`/`branch_id` exist AND belong to the same company,
    and rejects self-parent; `POST /departments/{id}/repoint` re-points links under the same rules.
  - Generic update/delete/upsert/bulk are **intentionally not mounted** on the guarded surface (v1).

The unguarded `routes()` / `create_organization_routes` remain for trusted/admin/seeding contexts,
but the extension guide names the guarded composition as the default for any real deployment.

### Accepted trade-off

The validated writers are hand-rolled outside `BackboneCrudHandler`, which the module CLAUDE.md
flags as an anti-pattern. This is a **deliberate exception**, recorded here so a future regen author
does not "fix" it back to open CRUD. Both files are `user_owned` in `metaphor.codegen.yaml`.

## Consequences

- The three probed holes are closed; four route-level integrity probes (`tests/integrity_probes.rs`)
  hit the router and lock the behavior against regression: company-write-locked, branch-bad-npwp
  rejected, cross-company-parent rejected, valid writes still 201.
- Residual (accepted v1 debt, see parking lot): NPWP validation is digit-count only (no mod-11
  checksum); NIB is unvalidated; department parent-chain cycles beyond self-parent are not yet
  guarded; branch/department generic update/delete are not on the guarded surface.
- Module maturity moves from "invariants enforced on one of thirteen write paths" to "enforced on
  every mounted write path."

## Parking lot (deferred)

- backbone-core `BackboneCrudHandler::upsert` conflict-target vs partial unique index (`ON CONFLICT
  (code)` vs `WHERE deleted_at IS NULL`) — scope: backbone-core.
- NPWP mod-11 checksum + NIB 13-digit validation.
- Full department-tree acyclicity guard (multi-level cycles), second-head-office guard.
- Verify downstream modules honor the logical FKs (two-sided contract).
