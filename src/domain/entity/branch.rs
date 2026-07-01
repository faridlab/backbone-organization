use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::BranchType;
use super::OrgStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Branch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BranchId(pub Uuid);

impl BranchId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for BranchId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for BranchId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<BranchId> for Uuid {
    fn from(id: BranchId) -> Self { id.0 }
}

impl AsRef<Uuid> for BranchId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for BranchId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Branch {
    pub id: Uuid,
    pub company_id: Uuid,
    pub code: String,
    pub name: String,
    pub branch_type: BranchType,
    pub is_head_office: bool,
    pub npwp: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub status: OrgStatus,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Branch {
    /// Create a builder for Branch
    pub fn builder() -> BranchBuilder {
        BranchBuilder::default()
    }

    /// Create a new Branch with required fields
    pub fn new(company_id: Uuid, code: String, name: String, branch_type: BranchType, is_head_office: bool, country: String, status: OrgStatus) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            code,
            name,
            branch_type,
            is_head_office,
            npwp: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            province: None,
            postal_code: None,
            country,
            status,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> BranchId {
        BranchId(self.id)
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
    pub fn status(&self) -> &OrgStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the npwp field (chainable)
    pub fn with_npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
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

    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.company_id = v; }
                }
                "code" => {
                    if let Ok(v) = serde_json::from_value(value) { self.code = v; }
                }
                "name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.name = v; }
                }
                "branch_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.branch_type = v; }
                }
                "is_head_office" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_head_office = v; }
                }
                "npwp" => {
                    if let Ok(v) = serde_json::from_value(value) { self.npwp = v; }
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
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Branch {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Branch"
    }
}

impl backbone_core::PersistentEntity for Branch {
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

impl backbone_orm::EntityRepoMeta for Branch {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("branch_type".to_string(), "branch_type".to_string());
        m.insert("status".to_string(), "org_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["code", "name", "country"]
    }
    fn relations() -> &'static [(&'static str, &'static str, &'static str)] {
        &[("company", "companies", "companyId")]
    }
}

/// Builder for Branch entity
///
/// Provides a fluent API for constructing Branch instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct BranchBuilder {
    company_id: Option<Uuid>,
    code: Option<String>,
    name: Option<String>,
    branch_type: Option<BranchType>,
    is_head_office: Option<bool>,
    npwp: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    city: Option<String>,
    province: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
    status: Option<OrgStatus>,
}

impl BranchBuilder {
    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the code field (required)
    pub fn code(mut self, value: String) -> Self {
        self.code = Some(value);
        self
    }

    /// Set the name field (required)
    pub fn name(mut self, value: String) -> Self {
        self.name = Some(value);
        self
    }

    /// Set the branch_type field (default: `BranchType::default()`)
    pub fn branch_type(mut self, value: BranchType) -> Self {
        self.branch_type = Some(value);
        self
    }

    /// Set the is_head_office field (default: `false`)
    pub fn is_head_office(mut self, value: bool) -> Self {
        self.is_head_office = Some(value);
        self
    }

    /// Set the npwp field (optional)
    pub fn npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
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

    /// Set the status field (default: `OrgStatus::default()`)
    pub fn status(mut self, value: OrgStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Build the Branch entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Branch, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let code = self.code.ok_or_else(|| "code is required".to_string())?;
        let name = self.name.ok_or_else(|| "name is required".to_string())?;

        Ok(Branch {
            id: Uuid::new_v4(),
            company_id,
            code,
            name,
            branch_type: self.branch_type.unwrap_or(BranchType::default()),
            is_head_office: self.is_head_office.unwrap_or(false),
            npwp: self.npwp,
            email: self.email,
            phone: self.phone,
            address: self.address,
            city: self.city,
            province: self.province,
            postal_code: self.postal_code,
            country: self.country.unwrap_or("ID".to_string()),
            status: self.status.unwrap_or(OrgStatus::default()),
            metadata: AuditMetadata::default(),
        })
    }
}
