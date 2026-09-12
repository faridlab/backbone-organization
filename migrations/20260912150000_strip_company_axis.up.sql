-- Strip the module-native company axis from the organization masters (ADR-0029).
--
-- The module is tenant-agnostic from here on: no scoping column, no module fence. A branch's
-- place in the org tree is its org_units node (id = branch id, kind 'branch'), minted by the
-- write paths in the same transaction as the row; every other per-unit master (departments,
-- levels, positions, structures, the KBLI classification) is placement-owned by the composing
-- service's tenancy decorator, which adds org_unit_id, the RLS policy, the kind guard, and the
-- per-unit uniques at composition time. companies, industries, and org_units carry no tenant
-- axis at all — they ARE the registry / a public classification / the scoping tree — and stay
-- intentionally unfenced under any declaration.
--
-- The legacy group-tree column companies.parent_company_id dies with the axis: the single tree
-- truth is org_units.parent_id (the spine already mirrored it, node ids preserving company ids),
-- and a compatibility view exposes the parent/subsidiary pairs.

-- 0. Guard: every remaining reference must resolve to a company node in org_units (the spine
--    minted one per companies row, preserving ids). A row pointing at a company with no node
--    could not be placed by any later decorator backfill — refuse rather than orphan it.
DO $$
DECLARE bad_count integer;
BEGIN
    SELECT COUNT(*) INTO bad_count FROM (
        SELECT company_id FROM organization.branches          WHERE company_id IS NOT NULL
        UNION ALL SELECT company_id FROM organization.departments         WHERE company_id IS NOT NULL
        UNION ALL SELECT company_id FROM organization.company_industries  WHERE company_id IS NOT NULL
        UNION ALL SELECT company_id FROM organization.levels              WHERE company_id IS NOT NULL
        UNION ALL SELECT company_id FROM organization.positions           WHERE company_id IS NOT NULL
        UNION ALL SELECT company_id FROM organization.structures          WHERE company_id IS NOT NULL
    ) refs
    WHERE refs.company_id NOT IN (SELECT id FROM organization.org_units WHERE kind = 'company');
    IF bad_count > 0 THEN
        RAISE EXCEPTION 'organization strip: % master row(s) reference a company with no org_units company node — place them before stripping', bad_count;
    END IF;
END $$;

-- 1. Retire the shared-tree fence. The policies go; the RLS flags STAY ARMED — the module
--    ships the half-fence (enabled + forced, zero policies): default-deny for any non-owner
--    role until the composing decorator installs the org-scope policies.
DROP POLICY IF EXISTS branches_company_isolation         ON organization.branches;
DROP POLICY IF EXISTS departments_company_isolation      ON organization.departments;
DROP POLICY IF EXISTS company_industries_company_isolation ON organization.company_industries;
DROP POLICY IF EXISTS levels_company_isolation           ON organization.levels;
DROP POLICY IF EXISTS positions_company_isolation        ON organization.positions;
DROP POLICY IF EXISTS structures_company_isolation       ON organization.structures;

DROP FUNCTION IF EXISTS organization.company_subtree(uuid);

-- 2. Company-leading indexes. Their guarantees are NOT silently dropped: the module schema
--    re-declares the tenant-agnostic residue (status / head-office / parent / code lookups)
--    and the composing decorator re-declares every uniqueness guarantee org-scoped.
DROP INDEX IF EXISTS organization.idx_branches_company_id_code;
DROP INDEX IF EXISTS organization.idx_branches_company_id_status;
DROP INDEX IF EXISTS organization.idx_branches_company_id_is_head_office;

DROP INDEX IF EXISTS organization.idx_departments_company_id_code;
DROP INDEX IF EXISTS organization.idx_departments_company_id_parent_id_sort_order;
DROP INDEX IF EXISTS organization.idx_departments_company_id_status;

DROP INDEX IF EXISTS organization.idx_company_industries_company_id_industry_id;
DROP INDEX IF EXISTS organization.idx_company_industries_company_id;
DROP INDEX IF EXISTS organization.idx_company_industries_company_id_is_primary;

DROP INDEX IF EXISTS organization.idx_levels_company_id_name;
DROP INDEX IF EXISTS organization.idx_levels_company_id_order_number;
DROP INDEX IF EXISTS organization.idx_levels_company_id;

DROP INDEX IF EXISTS organization.idx_positions_company_id_code;
DROP INDEX IF EXISTS organization.idx_positions_company_id_name;
DROP INDEX IF EXISTS organization.idx_positions_company_id;

DROP INDEX IF EXISTS organization.idx_structures_company_id_parent_id;
DROP INDEX IF EXISTS organization.idx_structures_company_id_name;

-- 3. Company foreign keys.
ALTER TABLE organization.branches         DROP CONSTRAINT IF EXISTS fk_branches_company_id;
ALTER TABLE organization.departments      DROP CONSTRAINT IF EXISTS fk_departments_company_id;
ALTER TABLE organization.company_industries DROP CONSTRAINT IF EXISTS fk_company_industries_company_id;
ALTER TABLE organization.levels           DROP CONSTRAINT IF EXISTS fk_levels_company_id;
ALTER TABLE organization.positions        DROP CONSTRAINT IF EXISTS fk_positions_company_id;
ALTER TABLE organization.structures       DROP CONSTRAINT IF EXISTS fk_structures_company_id;

-- 4. The axis itself.
ALTER TABLE organization.branches         DROP COLUMN IF EXISTS company_id;
ALTER TABLE organization.departments      DROP COLUMN IF EXISTS company_id;
ALTER TABLE organization.company_industries DROP COLUMN IF EXISTS company_id;
ALTER TABLE organization.levels           DROP COLUMN IF EXISTS company_id;
ALTER TABLE organization.positions        DROP COLUMN IF EXISTS company_id;
ALTER TABLE organization.structures       DROP COLUMN IF EXISTS company_id;

-- 5. companies: the legacy group-tree column dies — org_units.parent_id is the single tree
--    truth; the view below keeps raw parent/subsidiary consumers working.
DROP INDEX IF EXISTS organization.idx_companies_parent_company_id;
ALTER TABLE organization.companies DROP COLUMN IF EXISTS parent_company_id;

-- 6. Compatibility view over the authoritative tree (one row per company-kind node with a
--    parent; ids are company ids by the spine's id-preserving contract).
CREATE OR REPLACE VIEW organization.company_subsidiaries AS
SELECT parent_id AS parent_unit_id,
       id        AS subsidiary_id
FROM organization.org_units
WHERE kind = 'company'
  AND parent_id IS NOT NULL;
