-- Best-effort restore of the module-native company axis (dev/inspection only — reverting is
-- NOT a supported production path; the forward decorator is the composed isolation story).
-- Columns come back NULLABLE and EMPTY: the strip deliberately does not preserve placement
-- data (dev data is disposable; see the strip migration's guard for why a value-restoring
-- reverse is impossible once the decorator has re-keyed placement).

DROP VIEW IF EXISTS organization.company_subsidiaries;

ALTER TABLE organization.branches         ADD COLUMN IF NOT EXISTS company_id UUID;
ALTER TABLE organization.departments      ADD COLUMN IF NOT EXISTS company_id UUID;
ALTER TABLE organization.company_industries ADD COLUMN IF NOT EXISTS company_id UUID;
ALTER TABLE organization.levels           ADD COLUMN IF NOT EXISTS company_id UUID;
ALTER TABLE organization.positions        ADD COLUMN IF NOT EXISTS company_id UUID;
ALTER TABLE organization.structures       ADD COLUMN IF NOT EXISTS company_id UUID;

ALTER TABLE organization.branches
    ADD CONSTRAINT fk_branches_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);
ALTER TABLE organization.departments
    ADD CONSTRAINT fk_departments_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);
ALTER TABLE organization.company_industries
    ADD CONSTRAINT fk_company_industries_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);
ALTER TABLE organization.levels
    ADD CONSTRAINT fk_levels_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);
ALTER TABLE organization.positions
    ADD CONSTRAINT fk_positions_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);
ALTER TABLE organization.structures
    ADD CONSTRAINT fk_structures_company_id FOREIGN KEY (company_id) REFERENCES organization.companies (id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_branches_company_id_code
    ON organization.branches (company_id, code) WHERE (metadata->>'deleted_at') IS NULL;
CREATE INDEX IF NOT EXISTS idx_branches_company_id_status
    ON organization.branches (company_id, status);
CREATE INDEX IF NOT EXISTS idx_branches_company_id_is_head_office
    ON organization.branches (company_id, is_head_office);

CREATE UNIQUE INDEX IF NOT EXISTS idx_departments_company_id_code
    ON organization.departments (company_id, code) WHERE (metadata->>'deleted_at') IS NULL;
CREATE INDEX IF NOT EXISTS idx_departments_company_id_parent_id_sort_order
    ON organization.departments (company_id, parent_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_departments_company_id_status
    ON organization.departments (company_id, status);

CREATE UNIQUE INDEX IF NOT EXISTS idx_company_industries_company_id_industry_id
    ON organization.company_industries (company_id, industry_id) WHERE (metadata->>'deleted_at') IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_company_industries_company_id
    ON organization.company_industries (company_id) WHERE is_primary = true AND (metadata->>'deleted_at') IS NULL;
CREATE INDEX IF NOT EXISTS idx_company_industries_company_id_is_primary
    ON organization.company_industries (company_id, is_primary);

CREATE UNIQUE INDEX IF NOT EXISTS idx_levels_company_id_name
    ON organization.levels (company_id, name) WHERE (metadata->>'deleted_at') IS NULL;
CREATE INDEX IF NOT EXISTS idx_levels_company_id_order_number
    ON organization.levels (company_id, order_number);
CREATE INDEX IF NOT EXISTS idx_levels_company_id ON organization.levels (company_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_positions_company_id_code
    ON organization.positions (company_id, code) WHERE code IS NOT NULL AND (metadata->>'deleted_at') IS NULL;
CREATE INDEX IF NOT EXISTS idx_positions_company_id_name ON organization.positions (company_id, name);
CREATE INDEX IF NOT EXISTS idx_positions_company_id ON organization.positions (company_id);

CREATE INDEX IF NOT EXISTS idx_structures_company_id_parent_id ON organization.structures (company_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_structures_company_id_name ON organization.structures (company_id, name);

ALTER TABLE organization.companies ADD COLUMN IF NOT EXISTS parent_company_id UUID;
CREATE INDEX IF NOT EXISTS idx_companies_parent_company_id ON organization.companies (parent_company_id);

-- The shared-tree fence returns: helper + one policy per fenced table, on the still-armed
-- RLS flags (the strip never disarmed them).
CREATE OR REPLACE FUNCTION organization.company_subtree(root uuid)
RETURNS TABLE (company_id uuid)
LANGUAGE sql
STABLE
AS $$
    WITH RECURSIVE tree AS (
        SELECT c.id AS node_id
        FROM organization.companies c
        WHERE c.id = root
        UNION ALL
        SELECT c.id
        FROM organization.companies c
        JOIN tree t ON c.parent_company_id = t.node_id
    )
    SELECT node_id FROM tree
$$;

CREATE POLICY branches_company_isolation ON organization.branches
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));

CREATE POLICY departments_company_isolation ON organization.departments
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));

CREATE POLICY company_industries_company_isolation ON organization.company_industries
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));

CREATE POLICY levels_company_isolation ON organization.levels
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));

CREATE POLICY positions_company_isolation ON organization.positions
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));

CREATE POLICY structures_company_isolation ON organization.structures
    FOR ALL
    USING      (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)))
    WITH CHECK (company_id IN (SELECT company_id FROM organization.company_subtree(NULLIF(current_setting('app.company_id', true), '')::uuid)));
