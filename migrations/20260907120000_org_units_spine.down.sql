-- Reverse the org-tree spine population (ADR-0028).
-- Drops the resolver helpers and the derived node rows; the org_units table
-- itself belongs to the create_org_unit migration.

DROP FUNCTION IF EXISTS organization.org_unit_root();
DROP FUNCTION IF EXISTS organization.org_unit_subtree(uuid[]);

DELETE FROM organization.org_units;
