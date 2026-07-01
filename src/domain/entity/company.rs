use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::CompanyEntityType;
use super::CompanyStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Company
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompanyId(pub Uuid);

impl CompanyId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for CompanyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for CompanyId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for CompanyId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<CompanyId> for Uuid {
    fn from(id: CompanyId) -> Self { id.0 }
}

impl AsRef<Uuid> for CompanyId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for CompanyId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Company {
    pub id: Uuid,
    pub code: String,
    pub legal_name: String,
    pub trade_name: Option<String>,
    pub npwp: Option<String>,
    pub nib: Option<String>,
    pub entity_type: CompanyEntityType,
    pub base_currency: String,
    pub fiscal_year_start_month: i32,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub parent_company_id: Option<Uuid>,
    pub is_default: bool,
    pub status: CompanyStatus,
    pub notes: Option<String>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Company {
    /// Create a builder for Company
    pub fn builder() -> CompanyBuilder {
        CompanyBuilder::default()
    }

    /// Create a new Company with required fields
    pub fn new(code: String, legal_name: String, entity_type: CompanyEntityType, base_currency: String, fiscal_year_start_month: i32, country: String, is_default: bool, status: CompanyStatus) -> Self {
        Self {
            id: Uuid::new_v4(),
            code,
            legal_name,
            trade_name: None,
            npwp: None,
            nib: None,
            entity_type,
            base_currency,
            fiscal_year_start_month,
            email: None,
            phone: None,
            address: None,
            city: None,
            province: None,
            postal_code: None,
            country,
            parent_company_id: None,
            is_default,
            status,
            notes: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> CompanyId {
        CompanyId(self.id)
    }

    /// Get when this entity was created
    pub fn created_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.created_at.as_ref()
    }

    /// Get when this entity was last updated
    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.updated_at.as_ref()
    }

    /// Check if this entity is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted_at.is_some()
    }

    /// Check if this entity is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.metadata.deleted_at.is_none()
    }

    /// Get when this entity was deleted
    pub fn deleted_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.deleted_at.as_ref()
    }

    /// Get who created this entity
    pub fn created_by(&self) -> Option<&Uuid> {
        self.metadata.created_by.as_ref()
    }

    /// Get who last updated this entity
    pub fn updated_by(&self) -> Option<&Uuid> {
        self.metadata.updated_by.as_ref()
    }

    /// Get who deleted this entity
    pub fn deleted_by(&self) -> Option<&Uuid> {
        self.metadata.deleted_by.as_ref()
    }

    /// Get the current status
    pub fn status(&self) -> &CompanyStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the trade_name field (chainable)
    pub fn with_trade_name(mut self, value: String) -> Self {
        self.trade_name = Some(value);
        self
    }

    /// Set the npwp field (chainable)
    pub fn with_npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
        self
    }

    /// Set the nib field (chainable)
    pub fn with_nib(mut self, value: String) -> Self {
        self.nib = Some(value);
        self
    }

    /// Set the email field (chainable)
    pub fn with_email(mut self, value: String) -> Self {
        self.email = Some(value);
        self
    }

    /// Set the phone field (chainable)
    pub fn with_phone(mut self, value: String) -> Self {
        self.phone = Some(value);
        self
    }

    /// Set the address field (chainable)
    pub fn with_address(mut self, value: String) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the city field (chainable)
    pub fn with_city(mut self, value: String) -> Self {
        self.city = Some(value);
        self
    }

    /// Set the province field (chainable)
    pub fn with_province(mut self, value: String) -> Self {
        self.province = Some(value);
        self
    }

    /// Set the postal_code field (chainable)
    pub fn with_postal_code(mut self, value: String) -> Self {
        self.postal_code = Some(value);
        self
    }

    /// Set the parent_company_id field (chainable)
    pub fn with_parent_company_id(mut self, value: Uuid) -> Self {
        self.parent_company_id = Some(value);
        self
    }

    /// Set the notes field (chainable)
    pub fn with_notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.code = v; }
                }
                "legal_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.legal_name = v; }
                }
                "trade_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.trade_name = v; }
                }
                "npwp" => {
                    if let Ok(v) = serde_json::from_value(value) { self.npwp = v; }
                }
                "nib" => {
                    if let Ok(v) = serde_json::from_value(value) { self.nib = v; }
                }
                "entity_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.entity_type = v; }
                }
                "base_currency" => {
                    if let Ok(v) = serde_json::from_value(value) { self.base_currency = v; }
                }
                "fiscal_year_start_month" => {
                    if let Ok(v) = serde_json::from_value(value) { self.fiscal_year_start_month = v; }
                }
                "email" => {
                    if let Ok(v) = serde_json::from_value(value) { self.email = v; }
                }
                "phone" => {
                    if let Ok(v) = serde_json::from_value(value) { self.phone = v; }
                }
                "address" => {
                    if let Ok(v) = serde_json::from_value(value) { self.address = v; }
                }
                "city" => {
                    if let Ok(v) = serde_json::from_value(value) { self.city = v; }
                }
                "province" => {
                    if let Ok(v) = serde_json::from_value(value) { self.province = v; }
                }
                "postal_code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.postal_code = v; }
                }
                "country" => {
                    if let Ok(v) = serde_json::from_value(value) { self.country = v; }
                }
                "parent_company_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.parent_company_id = v; }
                }
                "is_default" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_default = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "notes" => {
                    if let Ok(v) = serde_json::from_value(value) { self.notes = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Company {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Company"
    }
}

impl backbone_core::PersistentEntity for Company {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.created_at
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.created_at = Some(ts);
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.updated_at
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.updated_at = Some(ts);
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.deleted_at
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        self.metadata.deleted_at = ts;
    }
}

impl backbone_orm::EntityRepoMeta for Company {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("parent_company_id".to_string(), "uuid".to_string());
        m.insert("entity_type".to_string(), "company_entity_type".to_string());
        m.insert("status".to_string(), "company_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["code", "legal_name", "base_currency", "country"]
    }
    fn relations() -> &'static [(&'static str, &'static str, &'static str)] {
        &[("parent", "companies", "parentCompanyId")]
    }
}

/// Builder for Company entity
///
/// Provides a fluent API for constructing Company instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct CompanyBuilder {
    code: Option<String>,
    legal_name: Option<String>,
    trade_name: Option<String>,
    npwp: Option<String>,
    nib: Option<String>,
    entity_type: Option<CompanyEntityType>,
    base_currency: Option<String>,
    fiscal_year_start_month: Option<i32>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    city: Option<String>,
    province: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
    parent_company_id: Option<Uuid>,
    is_default: Option<bool>,
    status: Option<CompanyStatus>,
    notes: Option<String>,
}

impl CompanyBuilder {
    /// Set the code field (required)
    pub fn code(mut self, value: String) -> Self {
        self.code = Some(value);
        self
    }

    /// Set the legal_name field (required)
    pub fn legal_name(mut self, value: String) -> Self {
        self.legal_name = Some(value);
        self
    }

    /// Set the trade_name field (optional)
    pub fn trade_name(mut self, value: String) -> Self {
        self.trade_name = Some(value);
        self
    }

    /// Set the npwp field (optional)
    pub fn npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
        self
    }

    /// Set the nib field (optional)
    pub fn nib(mut self, value: String) -> Self {
        self.nib = Some(value);
        self
    }

    /// Set the entity_type field (default: `CompanyEntityType::default()`)
    pub fn entity_type(mut self, value: CompanyEntityType) -> Self {
        self.entity_type = Some(value);
        self
    }

    /// Set the base_currency field (default: `"IDR".to_string()`)
    pub fn base_currency(mut self, value: String) -> Self {
        self.base_currency = Some(value);
        self
    }

    /// Set the fiscal_year_start_month field (default: `1`)
    pub fn fiscal_year_start_month(mut self, value: i32) -> Self {
        self.fiscal_year_start_month = Some(value);
        self
    }

    /// Set the email field (optional)
    pub fn email(mut self, value: String) -> Self {
        self.email = Some(value);
        self
    }

    /// Set the phone field (optional)
    pub fn phone(mut self, value: String) -> Self {
        self.phone = Some(value);
        self
    }

    /// Set the address field (optional)
    pub fn address(mut self, value: String) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the city field (optional)
    pub fn city(mut self, value: String) -> Self {
        self.city = Some(value);
        self
    }

    /// Set the province field (optional)
    pub fn province(mut self, value: String) -> Self {
        self.province = Some(value);
        self
    }

    /// Set the postal_code field (optional)
    pub fn postal_code(mut self, value: String) -> Self {
        self.postal_code = Some(value);
        self
    }

    /// Set the country field (default: `"ID".to_string()`)
    pub fn country(mut self, value: String) -> Self {
        self.country = Some(value);
        self
    }

    /// Set the parent_company_id field (optional)
    pub fn parent_company_id(mut self, value: Uuid) -> Self {
        self.parent_company_id = Some(value);
        self
    }

    /// Set the is_default field (default: `false`)
    pub fn is_default(mut self, value: bool) -> Self {
        self.is_default = Some(value);
        self
    }

    /// Set the status field (default: `CompanyStatus::default()`)
    pub fn status(mut self, value: CompanyStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the notes field (optional)
    pub fn notes(mut self, value: String) -> Self {
        self.notes = Some(value);
        self
    }

    /// Build the Company entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Company, String> {
        let code = self.code.ok_or_else(|| "code is required".to_string())?;
        let legal_name = self.legal_name.ok_or_else(|| "legal_name is required".to_string())?;

        Ok(Company {
            id: Uuid::new_v4(),
            code,
            legal_name,
            trade_name: self.trade_name,
            npwp: self.npwp,
            nib: self.nib,
            entity_type: self.entity_type.unwrap_or(CompanyEntityType::default()),
            base_currency: self.base_currency.unwrap_or("IDR".to_string()),
            fiscal_year_start_month: self.fiscal_year_start_month.unwrap_or(1),
            email: self.email,
            phone: self.phone,
            address: self.address,
            city: self.city,
            province: self.province,
            postal_code: self.postal_code,
            country: self.country.unwrap_or("ID".to_string()),
            parent_company_id: self.parent_company_id,
            is_default: self.is_default.unwrap_or(false),
            status: self.status.unwrap_or(CompanyStatus::default()),
            notes: self.notes,
            metadata: AuditMetadata::default(),
        })
    }
}
