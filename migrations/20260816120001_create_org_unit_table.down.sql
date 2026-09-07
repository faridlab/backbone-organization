-- Down: drop organization.org_units table
DROP TABLE IF EXISTS organization.org_units CASCADE;
DROP FUNCTION IF EXISTS organization.org_units_audit_timestamp() CASCADE;
