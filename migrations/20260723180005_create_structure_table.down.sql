-- Down: drop organization.structures table
DROP TABLE IF EXISTS organization.structures CASCADE;
DROP FUNCTION IF EXISTS organization.structures_audit_timestamp() CASCADE;
