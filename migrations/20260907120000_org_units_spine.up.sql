-- Org-tree scoping spine (ADR-0028): populate organization.org_units from the
-- existing companies and branches masters, PRESERVING UUIDs — a company's node
-- id IS the company id, a branch's node id IS the branch id — so every logical
-- company_id reference in consuming modules already points at a valid node and
-- the re-key to org_unit_id is a copy, not a remap.
--
-- The tree is the scoping substrate the request-scope resolver reads on every
-- request, inside the tenant-isolated database (ADR-0027); org_units itself is
-- intentionally unfenced (the tree is what scopes everything else).
--
-- companies.parent_company_id and branches.company_id remain as legacy columns
-- for this migration's data source only; org_units.parent_id is the
-- authoritative tree from here on.

-- 1. The tenant root — exactly one per database (partial unique index guards it).
INSERT INTO organization.org_units (id, kind, parent_id, code, name, metadata)
SELECT gen_random_uuid(), 'root', NULL, 'root', 'Tenant root',
       '{"created_at":null,"updated_at":null,"deleted_at":null,"created_by":null,"updated_by":null,"deleted_by":null}'::jsonb
WHERE NOT EXISTS (SELECT 1 FROM organization.org_units WHERE kind = 'root');

-- 2. Companies become company-kind nodes. Two phases so parent links resolve
--    regardless of insertion order under the self foreign key: first parent
--    everything at the root, then re-point to the parent company's node.
INSERT INTO organization.org_units (id, kind, parent_id, code, name, metadata)
SELECT c.id, 'company', r.id, c.code,
       COALESCE(NULLIF(c.trade_name, ''), c.legal_name),
       c.metadata
FROM organization.companies c
CROSS JOIN (SELECT id FROM organization.org_units WHERE kind = 'root') r
WHERE NOT EXISTS (SELECT 1 FROM organization.org_units u WHERE u.id = c.id);

UPDATE organization.org_units u
SET parent_id = c.parent_company_id
FROM organization.companies c
WHERE u.id = c.id
  AND u.kind = 'company'
  AND c.parent_company_id IS NOT NULL
  AND c.parent_company_id <> u.id;

-- 3. Branches become branch-kind nodes under their company's node. An orphan
--    branch (company row missing) is parked at the root rather than dropped —
--    loud in the tree, recoverable, never silently deleted.
INSERT INTO organization.org_units (id, kind, parent_id, code, name, metadata)
SELECT b.id, 'branch',
       COALESCE(u.id, r.id),
       b.code, b.name, b.metadata
FROM organization.branches b
CROSS JOIN (SELECT id FROM organization.org_units WHERE kind = 'root') r
LEFT JOIN organization.org_units u ON u.id = b.company_id AND u.kind = 'company'
WHERE NOT EXISTS (SELECT 1 FROM organization.org_units o WHERE o.id = b.id);

-- 4. Scope-resolution helpers the request scope calls on every request.
--    org_unit_subtree: the union of subtrees under the given nodes (soft-deleted
--    nodes excluded; a deleted branch hides its subtree — matches the fence's
--    intent). org_unit_root: the node every entitlement includes so tenant-wide
--    shared rows are always visible.

CREATE OR REPLACE FUNCTION organization.org_unit_subtree(p_roots uuid[])
RETURNS SETOF uuid
LANGUAGE sql STABLE PARALLEL SAFE AS $$
    WITH RECURSIVE tree AS (
        SELECT o.id
        FROM organization.org_units o
        WHERE o.id = ANY(p_roots)
          AND (o.metadata->>'deleted_at') IS NULL
        UNION ALL
        SELECT o.id
        FROM organization.org_units o
        JOIN tree t ON o.parent_id = t.id
        WHERE (o.metadata->>'deleted_at') IS NULL
    )
    SELECT id FROM tree
$$;

CREATE OR REPLACE FUNCTION organization.org_unit_root()
RETURNS uuid
LANGUAGE sql STABLE PARALLEL SAFE AS $$
    SELECT id FROM organization.org_units WHERE kind = 'root'
$$;
