use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::KBLISector;
use super::OrgStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Industry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IndustryId(pub Uuid);

impl IndustryId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for IndustryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for IndustryId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for IndustryId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<IndustryId> for Uuid {
    fn from(id: IndustryId) -> Self { id.0 }
}

impl AsRef<Uuid> for IndustryId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for IndustryId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Industry {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub sector: KBLISector,
    pub parent_id: Option<Uuid>,
    pub status: OrgStatus,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Industry {
    /// Create a builder for Industry
    pub fn builder() -> IndustryBuilder {
        IndustryBuilder::default()
    }

    /// Create a new Industry with required fields
    pub fn new(code: String, name: String, sector: KBLISector, status: OrgStatus) -> Self {
        Self {
            id: Uuid::new_v4(),
            code,
            name,
            sector,
            parent_id: None,
            status,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> IndustryId {
        IndustryId(self.id)
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

    /// Set the parent_id field (chainable)
    pub fn with_parent_id(mut self, value: Uuid) -> Self {
        self.parent_id = Some(value);
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
                "name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.name = v; }
                }
                "sector" => {
                    if let Ok(v) = serde_json::from_value(value) { self.sector = v; }
                }
                "parent_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.parent_id = v; }
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

impl super::Entity for Industry {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Industry"
    }
}

impl backbone_core::PersistentEntity for Industry {
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

impl backbone_orm::EntityRepoMeta for Industry {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("parent_id".to_string(), "uuid".to_string());
        m.insert("sector".to_string(), "kbli_sector".to_string());
        m.insert("status".to_string(), "org_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["code", "name"]
    }
    fn relations() -> &'static [(&'static str, &'static str, &'static str)] {
        &[("parent", "industries", "parentId")]
    }
}

/// Builder for Industry entity
///
/// Provides a fluent API for constructing Industry instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct IndustryBuilder {
    code: Option<String>,
    name: Option<String>,
    sector: Option<KBLISector>,
    parent_id: Option<Uuid>,
    status: Option<OrgStatus>,
}

impl IndustryBuilder {
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

    /// Set the sector field (required)
    pub fn sector(mut self, value: KBLISector) -> Self {
        self.sector = Some(value);
        self
    }

    /// Set the parent_id field (optional)
    pub fn parent_id(mut self, value: Uuid) -> Self {
        self.parent_id = Some(value);
        self
    }

    /// Set the status field (default: `OrgStatus::default()`)
    pub fn status(mut self, value: OrgStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Build the Industry entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Industry, String> {
        let code = self.code.ok_or_else(|| "code is required".to_string())?;
        let name = self.name.ok_or_else(|| "name is required".to_string())?;
        let sector = self.sector.ok_or_else(|| "sector is required".to_string())?;

        Ok(Industry {
            id: Uuid::new_v4(),
            code,
            name,
            sector,
            parent_id: self.parent_id,
            status: self.status.unwrap_or(OrgStatus::default()),
            metadata: AuditMetadata::default(),
        })
    }
}
