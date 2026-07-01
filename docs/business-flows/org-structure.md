# Business Flow — Org Structure (Branch & Department)

> Owning module: `backbone-organization` · Implemented in
> `src/application/service/org_write_service.rs`, enforced by
> `src/presentation/http/guarded_routes.rs`, proven by `tests/integrity_probes.rs`.
> Rules: R5–R7 in `schema/hooks/organization.hook.yaml`.

How an established company grows its structure: additional branches (cabang) and a department tree.
All writes go through the **validated** guarded path — the generic CRUD create is not mounted.

## Add a branch
- `POST /branches` with `{ companyId, code, name, branchType?, isHeadOffice?, npwp?, ... }`.
- Rules: **R1** NPWP format if present → `invalid_npwp`; **R5** company must exist →
  `company_not_found`. Success → `201 { id }`.
- A branch may carry its own NPWP (cabang registered separately for local tax reporting).

## Add a department
- `POST /departments` with `{ companyId, code, name, parentId?, branchId?, isGroup?, managerId? }`.
- Rules: company must exist (`company_not_found`); **R6** `parentId` (if present) must exist and
  belong to the same company (`parent_different_company`); **R7** `branchId` (if present) must
  belong to the same company (`branch_different_company`). Success → `201 { id }`.
- `managerId` is a logical reference to `sapiens.User` (no DB FK).

## Re-point a department
- `POST /departments/{id}/repoint` with `{ parentId?, branchId? }`.
- Same R6/R7 checks against the department's own company, plus **R8** no self-parent
  (`self_parent`).

## Status lifecycle
- **Company**: `active → inactive → suspended → dissolved`. `dissolved` is terminal (R9 — no
  un-dissolve).
- **Branch / Department**: `active ↔ inactive`.
- Declared as state machines in `schema/hooks/organization.hook.yaml`.

## Not yet enforced (parking lot)
- Multi-level department-tree cycle detection (only direct self-parent is guarded today).
- Exactly-one-head-office-per-company (onboarding creates one; a second could be added directly).
- NPWP mod-11 checksum and NIB 13-digit validation (format depth).
