-- Down: drop organization.industries table
DROP TABLE IF EXISTS organization.industries CASCADE;
DROP FUNCTION IF EXISTS organization.industries_audit_timestamp() CASCADE;
