-- Down: drop the declared foreign keys for organization module
ALTER TABLE IF EXISTS organization.org_units DROP CONSTRAINT IF EXISTS fk_org_units_parent_id;
ALTER TABLE IF EXISTS organization.company_industries DROP CONSTRAINT IF EXISTS fk_company_industries_industry_id;
