-- Down: drop organization.departments table
DROP TABLE IF EXISTS organization.departments CASCADE;
DROP FUNCTION IF EXISTS organization.departments_audit_timestamp() CASCADE;
