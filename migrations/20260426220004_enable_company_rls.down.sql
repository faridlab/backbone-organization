-- Down: remove the company RLS fence for organization module

-- Reverse the company RLS fence for organization.branches
DROP POLICY IF EXISTS branches_company_isolation ON organization.branches;
ALTER TABLE organization.branches NO FORCE ROW LEVEL SECURITY;
ALTER TABLE organization.branches DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for organization.company_industries
DROP POLICY IF EXISTS company_industries_company_isolation ON organization.company_industries;
ALTER TABLE organization.company_industries NO FORCE ROW LEVEL SECURITY;
ALTER TABLE organization.company_industries DISABLE ROW LEVEL SECURITY;

-- Reverse the company RLS fence for organization.departments
DROP POLICY IF EXISTS departments_company_isolation ON organization.departments;
ALTER TABLE organization.departments NO FORCE ROW LEVEL SECURITY;
ALTER TABLE organization.departments DISABLE ROW LEVEL SECURITY;

