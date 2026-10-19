-- Down: drop the declared rule constraints for organization module
ALTER TABLE IF EXISTS organization.structures DROP CONSTRAINT IF EXISTS ck_structures_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.structures DROP CONSTRAINT IF EXISTS ck_structures_name_required__required;
ALTER TABLE IF EXISTS organization.positions DROP CONSTRAINT IF EXISTS ck_positions_description_max_length__max_length;
ALTER TABLE IF EXISTS organization.positions DROP CONSTRAINT IF EXISTS ck_positions_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.positions DROP CONSTRAINT IF EXISTS ck_positions_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.positions DROP CONSTRAINT IF EXISTS ck_positions_name_required__required;
ALTER TABLE IF EXISTS organization.org_units DROP CONSTRAINT IF EXISTS ck_org_units_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.org_units DROP CONSTRAINT IF EXISTS ck_org_units_name_required__required;
ALTER TABLE IF EXISTS organization.org_units DROP CONSTRAINT IF EXISTS ck_org_units_code_max_length__max_length;
DO $$
BEGIN
    IF to_regclass('organization.uq_org_units_kind__unique') IS NOT NULL AND to_regclass('organization.idx_org_units_kind') IS NULL THEN
        ALTER INDEX organization.uq_org_units_kind__unique RENAME TO idx_org_units_kind;
    END IF;
END $$;

ALTER TABLE IF EXISTS organization.levels DROP CONSTRAINT IF EXISTS ck_levels_grade_max_length__max_length;
ALTER TABLE IF EXISTS organization.levels DROP CONSTRAINT IF EXISTS ck_levels_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.levels DROP CONSTRAINT IF EXISTS ck_levels_name_required__required;
ALTER TABLE IF EXISTS organization.industries DROP CONSTRAINT IF EXISTS ck_industries_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.industries DROP CONSTRAINT IF EXISTS ck_industries_name_required__required;
ALTER TABLE IF EXISTS organization.industries DROP CONSTRAINT IF EXISTS ck_industries_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.industries DROP CONSTRAINT IF EXISTS ck_industries_code_required__required;
DO $$
BEGIN
    IF to_regclass('organization.uq_industries_code__unique') IS NOT NULL AND to_regclass('organization.idx_industries_code') IS NULL THEN
        ALTER INDEX organization.uq_industries_code__unique RENAME TO idx_industries_code;
    END IF;
END $$;

ALTER TABLE IF EXISTS organization.departments DROP CONSTRAINT IF EXISTS ck_departments_level_non_negative__non_negative;
ALTER TABLE IF EXISTS organization.departments DROP CONSTRAINT IF EXISTS ck_departments_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.departments DROP CONSTRAINT IF EXISTS ck_departments_name_required__required;
ALTER TABLE IF EXISTS organization.departments DROP CONSTRAINT IF EXISTS ck_departments_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.departments DROP CONSTRAINT IF EXISTS ck_departments_code_required__required;
ALTER TABLE IF EXISTS organization.company_industries DROP CONSTRAINT IF EXISTS ck_company_industries_industry_id_required__required;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_notes_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_country_length__length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_postal_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_province_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_city_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_address_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_phone_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_email_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_fiscal_year_start_month_max__max;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_fiscal_year_start_month_min__min;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_base_currency_length__length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_nib_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_npwp_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_trade_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_legal_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_legal_name_required__required;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.companies DROP CONSTRAINT IF EXISTS ck_companies_code_required__required;
DO $$
BEGIN
    IF to_regclass('organization.uq_companies_npwp__unique') IS NOT NULL AND to_regclass('organization.idx_companies_npwp') IS NULL THEN
        ALTER INDEX organization.uq_companies_npwp__unique RENAME TO idx_companies_npwp;
    END IF;
END $$;

DO $$
BEGIN
    IF to_regclass('organization.uq_companies_code__unique') IS NOT NULL AND to_regclass('organization.idx_companies_code') IS NULL THEN
        ALTER INDEX organization.uq_companies_code__unique RENAME TO idx_companies_code;
    END IF;
END $$;

ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_country_length__length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_postal_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_province_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_city_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_address_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_phone_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_email_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_npwp_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_name_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_name_required__required;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_code_max_length__max_length;
ALTER TABLE IF EXISTS organization.branches DROP CONSTRAINT IF EXISTS ck_branches_code_required__required;
