-- Down: drop organization.branches table
DROP TABLE IF EXISTS organization.branches CASCADE;
DROP FUNCTION IF EXISTS organization.branches_audit_timestamp() CASCADE;
