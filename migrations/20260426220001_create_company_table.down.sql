-- Down: drop organization.companies table
DROP TABLE IF EXISTS organization.companies CASCADE;
DROP FUNCTION IF EXISTS organization.companies_audit_timestamp() CASCADE;
