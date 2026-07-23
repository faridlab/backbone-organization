use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use super::AuditMetadata;

/// Strongly-typed ID for CompanyIndustry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompanyIndustryId(pub Uuid);

impl CompanyIndustryId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for CompanyIndustryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for CompanyIndustryId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for CompanyIndustryId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<CompanyIndustryId> for Uuid {
    fn from(id: CompanyIndustryId) -> Self { id.0 }
}

impl AsRef<Uuid> for CompanyIndustryId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for CompanyIndustryId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompanyIndustry {
    pub id: Uuid,
    pub company_id: Uuid,
    pub industry_id: Uuid,
    pub is_primary: bool,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl CompanyIndustry {
    /// Create a builder for CompanyIndustry
    pub fn builder() -> CompanyIndustryBuilder {
        CompanyIndustryBuilder::default()
    }

    /// Create a new CompanyIndustry with required fields
    pub fn new(company_id: Uuid, industry_id: Uuid, is_primary: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            industry_id,
            is_primary,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> CompanyIndustryId {
        CompanyIndustryId(self.id)
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
                "industry_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.industry_id = v; }
                }
                "is_primary" => {
                    if let Ok(v) = serde_json::from_value(value) { self.is_primary = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for CompanyIndustry {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "CompanyIndustry"
    }
}

impl backbone_core::PersistentEntity for CompanyIndustry {
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

impl backbone_orm::EntityRepoMeta for CompanyIndustry {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("industry_id".to_string(), "uuid".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
    fn relations() -> &'static [(&'static str, &'static str, &'static str)] {
        &[("company", "companies", "companyId"), ("industry", "industries", "industryId")]
    }
}

/// Builder for CompanyIndustry entity
///
/// Provides a fluent API for constructing CompanyIndustry instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct CompanyIndustryBuilder {
    company_id: Option<Uuid>,
    industry_id: Option<Uuid>,
    is_primary: Option<bool>,
}

impl CompanyIndustryBuilder {
    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the industry_id field (required)
    pub fn industry_id(mut self, value: Uuid) -> Self {
        self.industry_id = Some(value);
        self
    }

    /// Set the is_primary field (default: `false`)
    pub fn is_primary(mut self, value: bool) -> Self {
        self.is_primary = Some(value);
        self
    }

    /// Build the CompanyIndustry entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<CompanyIndustry, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let industry_id = self.industry_id.ok_or_else(|| "industry_id is required".to_string())?;

        Ok(CompanyIndustry {
            id: Uuid::new_v4(),
            company_id,
            industry_id,
            is_primary: self.is_primary.unwrap_or(false),
            metadata: AuditMetadata::default(),
        })
    }
}
