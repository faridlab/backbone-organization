-- Down: drop organization.company_industries table
DROP TABLE IF EXISTS organization.company_industries CASCADE;
DROP FUNCTION IF EXISTS organization.company_industries_audit_timestamp() CASCADE;
