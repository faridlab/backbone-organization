# Organization acceptance oracle — backbone-organization
# Flow maps:   docs/business-flows/onboarding.md, docs/business-flows/org-structure.md
# Golden cases: docs/business-flows/golden-cases.md
# Declarative, business-level. Executable truth lives in tests/onboarding_golden_cases.rs
# and tests/integrity_probes.rs.

Feature: Onboard a company and grow its org structure
  In order to give every ERP module a stable Company/Branch/Department to reference
  As a tenant admin
  I want to onboard a company atomically and add branches/departments under validated rules

  Background:
    Given the tenant schema "organization" is migrated

  # ---------------------------------------------------------------------------
  # Onboarding
  # ---------------------------------------------------------------------------
  @happy-path @module:organization @ogc-1
  Scenario: Onboard a company creates a head-office branch atomically
    When I onboard company "ACME" with legal name "PT Acme Indonesia" and NPWP "01.234.567.8-901.000"
    Then the company is created with status "active", base currency "IDR", entity type "pt"
    And exactly one branch exists for it, of type "head_office" with is_head_office true
    And the response returns both companyId and hqBranchId

  @validation @module:organization @ogc-2
  Scenario: A malformed NPWP is rejected and nothing is written
    When I onboard company "BADNPWP" with NPWP "12345"
    Then the request is rejected with "invalid_npwp"
    And no company named "BADNPWP" exists

  @validation @module:organization @ogc-3
  Scenario: A duplicate company code is rejected
    Given a company "DUP" already exists
    When I onboard another company with code "DUP"
    Then the request is rejected with "duplicate_company_code"

  @validation @module:organization @ogc-3b
  Scenario: Reusing an NPWP on a new code is rejected
    Given a company exists with NPWP "012345678901234"
    When I onboard a company with a new code but the same NPWP "012345678901234"
    Then the request is rejected with "duplicate_npwp"

  @concurrency @module:organization @ogc-4
  Scenario: Two concurrent onboards of the same code yield exactly one company
    When two onboards of code "RACE" run concurrently
    Then exactly one succeeds
    And the database holds exactly one company with code "RACE"

  # ---------------------------------------------------------------------------
  # Guarded write path (org structure)
  # ---------------------------------------------------------------------------
  @guard @module:organization @igc-1
  Scenario: The generic company create route is not exposed on the guarded surface
    When I POST to "/companies" on the guarded routes
    Then the response status is 405 or 404

  @validation @module:organization @igc-2
  Scenario: A branch with a malformed NPWP is rejected
    Given a company "OKC" exists
    When I add a branch to "OKC" with NPWP "12345"
    Then the request is rejected with status 422

  @validation @module:organization @igc-3
  Scenario: A department whose parent belongs to another company is rejected
    Given a company "HOST" and a company "OTHER" each exist
    And "OTHER" has a department "ROOT"
    When I add a department to "HOST" with parent "ROOT"
    Then the request is rejected with status 422

  @happy-path @module:organization @igc-4
  Scenario: Valid branch and same-company department are created
    Given a company "OKC" exists
    When I add a branch to "OKC" with a valid 15-digit NPWP
    And I add a department to "OKC" under a same-company parent
    Then both are created with status 201
