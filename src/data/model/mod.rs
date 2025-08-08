//! # Data Models
//!
//! This module defines the core data structures for BLS data processing.
//! All models support serialization/deserialization and validation.
//!
//! ## Core Models
//!
//! - [`Series`]: Represents a BLS data series with metadata
//! - [`Observation`]: Represents a single data observation/value
//! - [`Lookup`]: Represents lookup table entries for code mappings
//! - [`Survey`]: Represents survey-level metadata and configuration
//!
//! ## Usage
//!
//! ```rust
//! use crate::data::model::{Series, Observation, Survey};
//!
//! // Create a series
//! let series = Series::new("APUS49074714", "Average Price Data - Gasoline");
//!
//! // Create an observation
//! let obs = Observation::new("APUS49074714", 2023, "M01", Some(3.45));
//!
//! // Create a survey
//! let survey = Survey::new("AP", "Average Price Data");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;
use chrono::{DateTime, Utc};
use crate::utils::validation::BLSValidationRules;

// Sub-module declarations
pub mod series;
pub mod observation;
pub mod lookup;
pub mod survey;

// Re-export all model types
pub use series::{Series, SeriesMetadata};
pub use observation::{Observation, ObservationValue};
pub use lookup::{Lookup, LookupEntry};
pub use survey::{Survey, SurveyMetadata};

/// Common metadata fields for all BLS data structures
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CommonMetadata {
    /// When this record was created
    #[serde(default = "current_timestamp")]
    pub created_at: DateTime<Utc>,
    
    /// When this record was last updated
    #[serde(default = "current_timestamp")]
    pub updated_at: DateTime<Utc>,
    
    /// Version of this record
    #[serde(default = "default_version")]
    pub version: u32,
    
    /// Source of this data
    #[serde(default)]
    pub source: Option<String>,
    
    /// Additional metadata as key-value pairs
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl CommonMetadata {
    /// Create new common metadata with current timestamp
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            source: None,
            attributes: HashMap::new(),
        }
    }

    /// Update the timestamp and increment version
    pub fn update(&mut self) {
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Set the source
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }
}

impl Default for CommonMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Data quality indicators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataQuality {
    /// High quality, reliable data
    High,
    /// Medium quality, some concerns
    Medium,
    /// Low quality, use with caution
    Low,
    /// Preliminary data, subject to revision
    Preliminary,
    /// Revised data
    Revised,
    /// Estimated data
    Estimated,
}

impl std::fmt::Display for DataQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataQuality::High => write!(f, "high"),
            DataQuality::Medium => write!(f, "medium"),
            DataQuality::Low => write!(f, "low"),
            DataQuality::Preliminary => write!(f, "preliminary"),
            DataQuality::Revised => write!(f, "revised"),
            DataQuality::Estimated => write!(f, "estimated"),
        }
    }
}

/// Data status indicators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataStatus {
    /// Active, current data
    Active,
    /// Inactive, no longer updated
    Inactive,
    /// Discontinued series
    Discontinued,
    /// Suspended temporarily
    Suspended,
}

impl std::fmt::Display for DataStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataStatus::Active => write!(f, "active"),
            DataStatus::Inactive => write!(f, "inactive"),
            DataStatus::Discontinued => write!(f, "discontinued"),
            DataStatus::Suspended => write!(f, "suspended"),
        }
    }
}

/// Frequency of data collection/publication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    /// Annual data
    Annual,
    /// Quarterly data
    Quarterly,
    /// Monthly data
    Monthly,
    /// Weekly data
    Weekly,
    /// Daily data
    Daily,
    /// Other frequency
    Other(String),
}

impl std::fmt::Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frequency::Annual => write!(f, "annual"),
            Frequency::Quarterly => write!(f, "quarterly"),
            Frequency::Monthly => write!(f, "monthly"),
            Frequency::Weekly => write!(f, "weekly"),
            Frequency::Daily => write!(f, "daily"),
            Frequency::Other(freq) => write!(f, "{}", freq),
        }
    }
}

/// Units of measurement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Unit {
    /// Unit code
    pub code: String,
    /// Unit name/description
    pub name: String,
    /// Unit symbol (e.g., "$", "%")
    pub symbol: Option<String>,
}

impl Unit {
    /// Create a new unit
    pub fn new(code: &str, name: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            symbol: None,
        }
    }

    /// Create a unit with symbol
    pub fn with_symbol(code: &str, name: &str, symbol: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            symbol: Some(symbol.to_string()),
        }
    }
}

/// Geographic area information
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Area {
    /// Area code
    #[validate(length(min = 1, max = 10))]
    pub code: String,
    
    /// Area name
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    
    /// Area type (e.g., "State", "MSA", "County")
    pub area_type: String,
    
    /// Parent area code (for hierarchical areas)
    pub parent_code: Option<String>,
}

impl Area {
    /// Create a new area
    pub fn new(code: &str, name: &str, area_type: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            area_type: area_type.to_string(),
            parent_code: None,
        }
    }

    /// Set parent area
    pub fn with_parent(mut self, parent_code: &str) -> Self {
        self.parent_code = Some(parent_code.to_string());
        self
    }
}

/// Industry/item information
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Item {
    /// Item code
    #[validate(length(min = 1, max = 20))]
    pub code: String,
    
    /// Item name/description
    #[validate(length(min = 1, max = 500))]
    pub name: String,
    
    /// Item category
    pub category: Option<String>,
    
    /// Parent item code (for hierarchical items)
    pub parent_code: Option<String>,
}

impl Item {
    /// Create a new item
    pub fn new(code: &str, name: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            category: None,
            parent_code: None,
        }
    }

    /// Set category
    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    /// Set parent item
    pub fn with_parent(mut self, parent_code: &str) -> Self {
        self.parent_code = Some(parent_code.to_string());
        self
    }
}

// Default value functions
fn current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

fn default_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_metadata() {
        let mut metadata = CommonMetadata::new();
        assert_eq!(metadata.version, 1);
        assert!(metadata.source.is_none());
        assert!(metadata.attributes.is_empty());

        metadata.update();
        assert_eq!(metadata.version, 2);
        assert!(metadata.updated_at > metadata.created_at);

        metadata.add_attribute("test_key", "test_value");
        assert_eq!(metadata.attributes.get("test_key"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_data_quality_display() {
        assert_eq!(DataQuality::High.to_string(), "high");
        assert_eq!(DataQuality::Preliminary.to_string(), "preliminary");
        assert_eq!(DataQuality::Revised.to_string(), "revised");
    }

    #[test]
    fn test_data_status_display() {
        assert_eq!(DataStatus::Active.to_string(), "active");
        assert_eq!(DataStatus::Discontinued.to_string(), "discontinued");
    }

    #[test]
    fn test_frequency_display() {
        assert_eq!(Frequency::Monthly.to_string(), "monthly");
        assert_eq!(Frequency::Other("biweekly".to_string()).to_string(), "biweekly");
    }

    #[test]
    fn test_unit() {
        let unit = Unit::new("USD", "US Dollars");
        assert_eq!(unit.code, "USD");
        assert_eq!(unit.name, "US Dollars");
        assert!(unit.symbol.is_none());

        let unit_with_symbol = Unit::with_symbol("PCT", "Percent", "%");
        assert_eq!(unit_with_symbol.symbol, Some("%".to_string()));
    }

    #[test]
    fn test_area() {
        let area = Area::new("US", "United States", "Country");
        assert_eq!(area.code, "US");
        assert_eq!(area.name, "United States");
        assert_eq!(area.area_type, "Country");
        assert!(area.parent_code.is_none());

        let state = Area::new("CA", "California", "State").with_parent("US");
        assert_eq!(state.parent_code, Some("US".to_string()));
    }

    #[test]
    fn test_item() {
        let item = Item::new("GASOLINE", "Gasoline, all types");
        assert_eq!(item.code, "GASOLINE");
        assert_eq!(item.name, "Gasoline, all types");
        assert!(item.category.is_none());

        let categorized_item = Item::new("REGULAR", "Regular gasoline")
            .with_category("Energy")
            .with_parent("GASOLINE");
        assert_eq!(categorized_item.category, Some("Energy".to_string()));
        assert_eq!(categorized_item.parent_code, Some("GASOLINE".to_string()));
    }

    #[test]
    fn test_serialization() {
        let metadata = CommonMetadata::new().with_source("BLS");
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: CommonMetadata = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(metadata.version, deserialized.version);
        assert_eq!(metadata.source, deserialized.source);
    }
}