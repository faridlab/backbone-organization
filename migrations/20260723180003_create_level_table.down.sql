-- Down: drop organization.levels table
DROP TABLE IF EXISTS organization.levels CASCADE;
DROP FUNCTION IF EXISTS organization.levels_audit_timestamp() CASCADE;
