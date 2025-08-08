//! # Lookup Data Model
//!
//! This module defines the Lookup data structure for BLS lookup tables.
//! Lookup tables map codes to human-readable descriptions and provide
//! hierarchical relationships between different categories.
//!
//! ## Usage
//!
//! ```rust
//! use crate::data::model::lookup::{Lookup, LookupEntry};
//!
//! // Create a new lookup table
//! let mut lookup = Lookup::new("area", "Geographic Areas");
//!
//! // Add entries
//! lookup.add_entry("US", "United States", None);
//! lookup.add_entry("CA", "California", Some("US"));
//!
//! // Query entries
//! let entry = lookup.get_entry("CA").unwrap();
//! assert_eq!(entry.description, "California");
//! ```

use crate::data::model::CommonMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// BLS lookup table
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Lookup {
    /// Lookup table identifier (e.g., "area", "item", "industry")
    #[validate(length(min = 1, max = 50))]
    pub table_id: String,

    /// Human-readable table name
    #[validate(length(min = 1, max = 200))]
    pub table_name: String,

    /// Survey code this lookup belongs to
    #[validate(length(min = 2, max = 2))]
    pub survey_code: String,

    /// Lookup entries indexed by code
    #[validate]
    pub entries: HashMap<String, LookupEntry>,

    /// Common metadata (timestamps, version, etc.)
    #[validate]
    pub common: CommonMetadata,
}

impl Lookup {
    /// Create a new lookup table
    pub fn new(table_id: &str, table_name: &str) -> Self {
        Self {
            table_id: table_id.to_string(),
            table_name: table_name.to_string(),
            survey_code: "XX".to_string(), // Default, should be set explicitly
            entries: HashMap::new(),
            common: CommonMetadata::new(),
        }
    }

    /// Create a lookup table for a specific survey
    pub fn for_survey(table_id: &str, table_name: &str, survey_code: &str) -> Self {
        Self {
            table_id: table_id.to_string(),
            table_name: table_name.to_string(),
            survey_code: survey_code.to_uppercase(),
            entries: HashMap::new(),
            common: CommonMetadata::new(),
        }
    }

    /// Add an entry to the lookup table
    pub fn add_entry(
        &mut self,
        code: &str,
        description: &str,
        parent_code: Option<&str>,
    ) -> &mut LookupEntry {
        let entry = LookupEntry::new(code, description, parent_code);
        self.entries.insert(code.to_string(), entry);
        self.common.update();
        self.entries.get_mut(code).unwrap()
    }

    /// Add a lookup entry object
    pub fn add_lookup_entry(&mut self, entry: LookupEntry) {
        let code = entry.code.clone();
        self.entries.insert(code, entry);
        self.common.update();
    }

    /// Get an entry by code
    pub fn get_entry(&self, code: &str) -> Option<&LookupEntry> {
        self.entries.get(code)
    }

    /// Get a mutable entry by code
    pub fn get_entry_mut(&mut self, code: &str) -> Option<&mut LookupEntry> {
        self.entries.get_mut(code)
    }

    /// Remove an entry by code
    pub fn remove_entry(&mut self, code: &str) -> Option<LookupEntry> {
        let result = self.entries.remove(code);
        if result.is_some() {
            self.common.update();
        }
        result
    }

    /// Check if an entry exists
    pub fn contains_entry(&self, code: &str) -> bool {
        self.entries.contains_key(code)
    }

    /// Get all entry codes
    pub fn get_codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = self.entries.keys().cloned().collect();
        codes.sort();
        codes
    }

    /// Get entries by parent code
    pub fn get_children(&self, parent_code: &str) -> Vec<&LookupEntry> {
        self.entries
            .values()
            .filter(|entry| entry.parent_code.as_deref() == Some(parent_code))
            .collect()
    }

    /// Get root entries (entries with no parent)
    pub fn get_roots(&self) -> Vec<&LookupEntry> {
        self.entries
            .values()
            .filter(|entry| entry.parent_code.is_none())
            .collect()
    }

    /// Get the full hierarchy path for an entry
    pub fn get_hierarchy_path(&self, code: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut current_code = Some(code.to_string());

        while let Some(code) = current_code {
            if let Some(entry) = self.get_entry(&code) {
                path.insert(0, code.clone());
                current_code = entry.parent_code.clone();
            } else {
                break;
            }
        }

        path
    }

    /// Get the full description path for an entry
    pub fn get_description_path(&self, code: &str) -> Vec<String> {
        self.get_hierarchy_path(code)
            .iter()
            .filter_map(|c| self.get_entry(c).map(|e| e.description.clone()))
            .collect()
    }

    /// Search entries by description (case-insensitive)
    pub fn search_by_description(&self, query: &str) -> Vec<&LookupEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|entry| entry.description.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if the lookup table is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.common.update();
    }

    /// Validate the lookup table structure
    pub fn validate_structure(&self) -> Result<(), String> {
        // Check for circular references
        for (code, entry) in &self.entries {
            if let Some(parent_code) = &entry.parent_code {
                if self.has_circular_reference(code, parent_code) {
                    return Err(format!("Circular reference detected for entry '{code}'"));
                }
            }
        }

        // Check for orphaned entries (parent doesn't exist)
        for (code, entry) in &self.entries {
            if let Some(parent_code) = &entry.parent_code {
                if !self.contains_entry(parent_code) {
                    return Err(format!(
                        "Entry '{code}' references non-existent parent '{parent_code}'"
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check for circular references in the hierarchy
    fn has_circular_reference(&self, start_code: &str, current_code: &str) -> bool {
        if start_code == current_code {
            return true;
        }

        if let Some(entry) = self.get_entry(current_code) {
            if let Some(parent_code) = &entry.parent_code {
                return self.has_circular_reference(start_code, parent_code);
            }
        }

        false
    }

    /// Get statistics about the lookup table
    pub fn get_statistics(&self) -> LookupStatistics {
        let total_entries = self.entries.len();
        let root_entries = self.get_roots().len();
        let max_depth = self.calculate_max_depth();
        let entries_with_children = self
            .entries
            .keys()
            .filter(|code| !self.get_children(code).is_empty())
            .count();

        LookupStatistics {
            total_entries,
            root_entries,
            max_depth,
            entries_with_children,
        }
    }

    /// Calculate the maximum depth of the hierarchy
    fn calculate_max_depth(&self) -> usize {
        self.entries
            .keys()
            .map(|code| self.get_hierarchy_path(code).len())
            .max()
            .unwrap_or(0)
    }
}

/// Lookup table entry
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LookupEntry {
    /// Entry code
    #[validate(length(min = 1, max = 50))]
    pub code: String,

    /// Human-readable description
    #[validate(length(min = 1, max = 500))]
    pub description: String,

    /// Parent entry code (for hierarchical lookups)
    pub parent_code: Option<String>,

    /// Entry status (active, inactive, etc.)
    #[serde(default = "default_active")]
    pub active: bool,

    /// Sort order for display
    pub sort_order: Option<u32>,

    /// Additional attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,

    /// Common metadata
    #[validate]
    pub common: CommonMetadata,
}

impl LookupEntry {
    /// Create a new lookup entry
    pub fn new(code: &str, description: &str, parent_code: Option<&str>) -> Self {
        Self {
            code: code.to_string(),
            description: description.to_string(),
            parent_code: parent_code.map(|s| s.to_string()),
            active: true,
            sort_order: None,
            attributes: HashMap::new(),
            common: CommonMetadata::new(),
        }
    }

    /// Create a lookup entry builder
    pub fn builder(code: &str, description: &str) -> LookupEntryBuilder {
        LookupEntryBuilder::new(code, description)
    }

    /// Set the parent code
    pub fn set_parent(&mut self, parent_code: Option<&str>) {
        self.parent_code = parent_code.map(|s| s.to_string());
        self.common.update();
    }

    /// Set the active status
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.common.update();
    }

    /// Set the sort order
    pub fn set_sort_order(&mut self, sort_order: u32) {
        self.sort_order = Some(sort_order);
        self.common.update();
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
        self.common.update();
    }

    /// Remove an attribute
    pub fn remove_attribute(&mut self, key: &str) -> Option<String> {
        let result = self.attributes.remove(key);
        if result.is_some() {
            self.common.update();
        }
        result
    }

    /// Check if entry has a specific attribute
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get an attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }
}

/// Builder for LookupEntry
pub struct LookupEntryBuilder {
    code: String,
    description: String,
    parent_code: Option<String>,
    active: bool,
    sort_order: Option<u32>,
    attributes: HashMap<String, String>,
}

impl LookupEntryBuilder {
    /// Create a new builder
    pub fn new(code: &str, description: &str) -> Self {
        Self {
            code: code.to_string(),
            description: description.to_string(),
            parent_code: None,
            active: true,
            sort_order: None,
            attributes: HashMap::new(),
        }
    }

    /// Set the parent code
    pub fn parent(mut self, parent_code: &str) -> Self {
        self.parent_code = Some(parent_code.to_string());
        self
    }

    /// Set the active status
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Set the sort order
    pub fn sort_order(mut self, sort_order: u32) -> Self {
        self.sort_order = Some(sort_order);
        self
    }

    /// Add an attribute
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// Build the lookup entry
    pub fn build(self) -> LookupEntry {
        LookupEntry {
            code: self.code,
            description: self.description,
            parent_code: self.parent_code,
            active: self.active,
            sort_order: self.sort_order,
            attributes: self.attributes,
            common: CommonMetadata::new(),
        }
    }
}

/// Statistics about a lookup table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupStatistics {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of root entries (no parent)
    pub root_entries: usize,
    /// Maximum depth of the hierarchy
    pub max_depth: usize,
    /// Number of entries that have children
    pub entries_with_children: usize,
}

// Default value functions
fn default_active() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_new() {
        let lookup = Lookup::new("area", "Geographic Areas");
        assert_eq!(lookup.table_id, "area");
        assert_eq!(lookup.table_name, "Geographic Areas");
        assert_eq!(lookup.survey_code, "XX");
        assert!(lookup.is_empty());
    }

    #[test]
    fn test_lookup_for_survey() {
        let lookup = Lookup::for_survey("area", "Geographic Areas", "ap");
        assert_eq!(lookup.survey_code, "AP");
    }

    #[test]
    fn test_add_and_get_entry() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));

        assert_eq!(lookup.entry_count(), 2);
        assert!(!lookup.is_empty());

        let us_entry = lookup.get_entry("US").unwrap();
        assert_eq!(us_entry.description, "United States");
        assert!(us_entry.parent_code.is_none());

        let ca_entry = lookup.get_entry("CA").unwrap();
        assert_eq!(ca_entry.description, "California");
        assert_eq!(ca_entry.parent_code, Some("US".to_string()));
    }

    #[test]
    fn test_hierarchy_operations() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));
        lookup.add_entry("LA", "Los Angeles", Some("CA"));
        lookup.add_entry("NY", "New York", Some("US"));

        // Test get_children
        let us_children = lookup.get_children("US");
        assert_eq!(us_children.len(), 2);
        assert!(us_children.iter().any(|e| e.code == "CA"));
        assert!(us_children.iter().any(|e| e.code == "NY"));

        let ca_children = lookup.get_children("CA");
        assert_eq!(ca_children.len(), 1);
        assert_eq!(ca_children[0].code, "LA");

        // Test get_roots
        let roots = lookup.get_roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].code, "US");

        // Test hierarchy path
        let la_path = lookup.get_hierarchy_path("LA");
        assert_eq!(la_path, vec!["US", "CA", "LA"]);

        let description_path = lookup.get_description_path("LA");
        assert_eq!(
            description_path,
            vec!["United States", "California", "Los Angeles"]
        );
    }

    #[test]
    fn test_search_by_description() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));
        lookup.add_entry("NY", "New York", Some("US"));

        let results = lookup.search_by_description("new");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].code, "NY");

        let results = lookup.search_by_description("states");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].code, "US");
    }

    #[test]
    fn test_lookup_entry_builder() {
        let entry = LookupEntry::builder("CA", "California")
            .parent("US")
            .active(true)
            .sort_order(1)
            .attribute("region", "West")
            .build();

        assert_eq!(entry.code, "CA");
        assert_eq!(entry.description, "California");
        assert_eq!(entry.parent_code, Some("US".to_string()));
        assert!(entry.active);
        assert_eq!(entry.sort_order, Some(1));
        assert_eq!(entry.get_attribute("region"), Some(&"West".to_string()));
    }

    #[test]
    fn test_lookup_entry_attributes() {
        let mut entry = LookupEntry::new("CA", "California", None);

        entry.add_attribute("region", "West");
        entry.add_attribute("population", "39538223");

        assert!(entry.has_attribute("region"));
        assert_eq!(entry.get_attribute("region"), Some(&"West".to_string()));

        let removed = entry.remove_attribute("population");
        assert_eq!(removed, Some("39538223".to_string()));
        assert!(!entry.has_attribute("population"));
    }

    #[test]
    fn test_validate_structure() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        // Valid structure
        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));
        assert!(lookup.validate_structure().is_ok());

        // Add orphaned entry
        lookup.add_entry("ORPHAN", "Orphaned Entry", Some("NONEXISTENT"));
        assert!(lookup.validate_structure().is_err());

        // Remove orphaned entry and add circular reference
        lookup.remove_entry("ORPHAN");
        lookup.add_entry("CIRCULAR", "Circular Entry", Some("CIRCULAR"));
        assert!(lookup.validate_structure().is_err());
    }

    #[test]
    fn test_lookup_statistics() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));
        lookup.add_entry("NY", "New York", Some("US"));
        lookup.add_entry("LA", "Los Angeles", Some("CA"));

        let stats = lookup.get_statistics();
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.root_entries, 1);
        assert_eq!(stats.max_depth, 3); // US -> CA -> LA
        assert_eq!(stats.entries_with_children, 2); // US and CA have children
    }

    #[test]
    fn test_lookup_operations() {
        let mut lookup = Lookup::new("area", "Geographic Areas");

        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", Some("US"));

        assert!(lookup.contains_entry("US"));
        assert!(lookup.contains_entry("CA"));
        assert!(!lookup.contains_entry("TX"));

        let codes = lookup.get_codes();
        assert_eq!(codes, vec!["CA", "US"]); // Sorted

        let removed = lookup.remove_entry("CA");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().description, "California");
        assert!(!lookup.contains_entry("CA"));

        lookup.clear();
        assert!(lookup.is_empty());
        assert_eq!(lookup.entry_count(), 0);
    }

    #[test]
    fn test_serialization() {
        let mut lookup = Lookup::new("area", "Geographic Areas");
        lookup.add_entry("US", "United States", None);

        let serialized = serde_json::to_string(&lookup).unwrap();
        let deserialized: Lookup = serde_json::from_str(&serialized).unwrap();

        assert_eq!(lookup.table_id, deserialized.table_id);
        assert_eq!(lookup.table_name, deserialized.table_name);
        assert_eq!(lookup.entry_count(), deserialized.entry_count());
        assert!(deserialized.contains_entry("US"));
    }
}
