-- Down: drop organization.positions table
DROP TABLE IF EXISTS organization.positions CASCADE;
DROP FUNCTION IF EXISTS organization.positions_audit_timestamp() CASCADE;
