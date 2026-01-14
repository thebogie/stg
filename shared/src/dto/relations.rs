use crate::models::relations::{PlayedAt, PlayedWith, ResultedIn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Data Transfer Object for Relation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RelationDto {
    /// Unique identifier for the relation (optional for creation, will be set by ArangoDB if empty)
    #[serde(default)]
    pub id: Uuid,

    /// ID of the source entity
    pub from_id: Uuid,

    /// ID of the target entity
    pub to_id: Uuid,

    /// Type of relation
    #[validate(length(min = 1))]
    pub label: String,
}

/// Data Transfer Object for PlayedAt relation
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PlayedAtDto {
    /// Relation's ID (optional for creation, will be set by ArangoDB if empty)
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_from")]
    pub from: String,
    #[serde(rename = "_to")]
    pub to: String,
    #[serde(rename = "_label")]
    pub label: String,
}

/// Data Transfer Object for PlayedWith relation
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PlayedWithDto {
    /// Relation's ID (optional for creation, will be set by ArangoDB if empty)
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_from")]
    pub from: String,
    #[serde(rename = "_to")]
    pub to: String,
    #[serde(rename = "_label")]
    pub label: String,
}

/// Data Transfer Object for ResultedIn relation
#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ResultedInDto {
    /// Relation's ID (optional for creation, will be set by ArangoDB if empty)
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(rename = "_from")]
    pub from: String,
    #[serde(rename = "_to")]
    pub to: String,
    #[serde(rename = "_label")]
    pub label: String,
    pub place: i32,
    pub result: String,
}

impl From<&RelationDto> for RelationDto {
    fn from(relation: &RelationDto) -> Self {
        relation.clone()
    }
}

impl From<&PlayedAt> for PlayedAtDto {
    fn from(edge: &PlayedAt) -> Self {
        Self {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
        }
    }
}

impl From<PlayedAtDto> for PlayedAt {
    fn from(dto: PlayedAtDto) -> Self {
        Self {
            id: dto.id,
            rev: String::new(), // Let ArangoDB set this
            from: dto.from,
            to: dto.to,
            label: dto.label,
        }
    }
}

impl From<&PlayedWith> for PlayedWithDto {
    fn from(edge: &PlayedWith) -> Self {
        Self {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
        }
    }
}

impl From<PlayedWithDto> for PlayedWith {
    fn from(dto: PlayedWithDto) -> Self {
        Self {
            id: dto.id,
            rev: String::new(), // Let ArangoDB set this
            from: dto.from,
            to: dto.to,
            label: dto.label,
        }
    }
}

impl From<&ResultedIn> for ResultedInDto {
    fn from(edge: &ResultedIn) -> Self {
        Self {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
            place: edge.place,
            result: edge.result.clone(),
        }
    }
}

impl From<ResultedInDto> for ResultedIn {
    fn from(dto: ResultedInDto) -> Self {
        Self {
            id: dto.id,
            rev: String::new(), // Let ArangoDB set this
            from: dto.from,
            to: dto.to,
            label: dto.label,
            place: dto.place,
            result: dto.result,
        }
    }
}

impl PlayedAtDto {
    pub fn update_edge(&self, edge: &mut PlayedAt) {
        edge.id = self.id.clone();
        edge.from = self.from.clone();
        edge.to = self.to.clone();
        edge.label = self.label.clone();
    }
}

impl PlayedWithDto {
    pub fn update_edge(&self, edge: &mut PlayedWith) {
        edge.id = self.id.clone();
        edge.from = self.from.clone();
        edge.to = self.to.clone();
        edge.label = self.label.clone();
    }
}

impl ResultedInDto {
    pub fn update_edge(&self, edge: &mut ResultedIn) {
        edge.id = self.id.clone();
        edge.from = self.from.clone();
        edge.to = self.to.clone();
        edge.label = self.label.clone();
        edge.place = self.place;
        edge.result = self.result.clone();
    }
}
