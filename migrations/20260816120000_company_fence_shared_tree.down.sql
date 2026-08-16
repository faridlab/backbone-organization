-- Down: revert the organization module to the strict fence (ADR-0014 rollback).
-- Restores the per-company predicate on every table this migration flipped or
-- fenced, and drops the subtree helper nothing else depends on.

ALTER TABLE organization.branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.branches FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS branches_company_isolation ON organization.branches;
CREATE POLICY branches_company_isolation ON organization.branches
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE organization.departments ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.departments FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS departments_company_isolation ON organization.departments;
CREATE POLICY departments_company_isolation ON organization.departments
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE organization.company_industries ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.company_industries FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS company_industries_company_isolation ON organization.company_industries;
CREATE POLICY company_industries_company_isolation ON organization.company_industries
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE organization.levels ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.levels FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS levels_company_isolation ON organization.levels;
CREATE POLICY levels_company_isolation ON organization.levels
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE organization.positions ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.positions FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS positions_company_isolation ON organization.positions;
CREATE POLICY positions_company_isolation ON organization.positions
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

ALTER TABLE organization.structures ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization.structures FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS structures_company_isolation ON organization.structures;
CREATE POLICY structures_company_isolation ON organization.structures
    FOR ALL
    USING      (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid)
    WITH CHECK (company_id = NULLIF(current_setting('app.company_id', true), '')::uuid);

DROP FUNCTION IF EXISTS organization.company_subtree(uuid);
