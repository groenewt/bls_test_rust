//! # Survey Data Model
//!
//! This module defines the Survey data structure for BLS surveys.
//! A survey represents a collection of related data series and provides
//! metadata about the survey program.
//!
//! ## Usage
//!
//! ```rust
//! use crate::rusty::data::model::survey::{Survey, SurveyMetadata};
//! use crate::rusty::data::model::{DataStatus, Frequency};
//!
//! // Create a new survey
//! let survey = Survey::new("AP", "Average Price Data");
//!
//! // Create survey with full metadata
//! let metadata = SurveyMetadata::builder()
//!     .description("Consumer price data for selected items")
//!     .frequency(Frequency::Monthly)
//!     .status(DataStatus::Active)
//!     .build();
//!
//! let survey = Survey::with_metadata("AP", "Average Price Data", metadata);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;
use chrono::{DateTime, Utc};
use crate::data::model::{CommonMetadata, DataStatus, Frequency};
use crate::utils::validation::BLSValidationRules;

/// BLS survey
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Survey {
    /// Survey code (e.g., "AP", "BD", "CE")
    #[validate(length(min = 2, max = 2))]
    #[validate(custom = "validate_survey_code")]
    pub survey_code: String,
    
    /// Survey name/title
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    
    /// Survey metadata
    #[validate]
    pub metadata: SurveyMetadata,
    
    /// Common metadata (timestamps, version, etc.)
    #[validate]
    pub common: CommonMetadata,
}

impl Survey {
    /// Create a new survey with minimal information
    pub fn new(survey_code: &str, name: &str) -> Self {
        Self {
            survey_code: survey_code.to_uppercase(),
            name: name.to_string(),
            metadata: SurveyMetadata::default(),
            common: CommonMetadata::new(),
        }
    }

    /// Create a survey with full metadata
    pub fn with_metadata(survey_code: &str, name: &str, metadata: SurveyMetadata) -> Self {
        Self {
            survey_code: survey_code.to_uppercase(),
            name: name.to_string(),
            metadata,
            common: CommonMetadata::new(),
        }
    }

    /// Get the survey code
    pub fn code(&self) -> &str {
        &self.survey_code
    }

    /// Get the survey name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if the survey is active
    pub fn is_active(&self) -> bool {
        self.metadata.status == DataStatus::Active
    }

    /// Get the survey description
    pub fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    /// Get the survey frequency
    pub fn frequency(&self) -> &Frequency {
        &self.metadata.frequency
    }

    /// Get the survey contact information
    pub fn contact(&self) -> Option<&SurveyContact> {
        self.metadata.contact.as_ref()
    }

    /// Update the survey metadata
    pub fn update_metadata(&mut self, metadata: SurveyMetadata) {
        self.metadata = metadata;
        self.common.update();
    }

    /// Set the survey status
    pub fn set_status(&mut self, status: DataStatus) {
        self.metadata.status = status;
        self.common.update();
    }

    /// Set the survey description
    pub fn set_description(&mut self, description: &str) {
        self.metadata.description = Some(description.to_string());
        self.common.update();
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.common.add_attribute(key, value);
    }

    /// Validate the survey
    pub fn validate_survey(&self) -> Result<(), String> {
        // Validate using BLS rules
        BLSValidationRules::validate_series_metadata(
            &format!("{}TEMP", self.survey_code),
            &self.name,
            &self.survey_code,
        ).map_err(|e| e.to_string())?;

        // Validate using validator crate
        self.validate().map_err(|e| format!("Validation error: {:?}", e))?;

        Ok(())
    }
}

/// Survey metadata containing detailed information about the survey
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SurveyMetadata {
    /// Survey description
    pub description: Option<String>,
    
    /// Survey status
    pub status: DataStatus,
    
    /// Primary data frequency
    pub frequency: Frequency,
    
    /// Survey contact information
    pub contact: Option<SurveyContact>,
    
    /// Survey start date
    pub start_date: Option<DateTime<Utc>>,
    
    /// Survey end date (if discontinued)
    pub end_date: Option<DateTime<Utc>>,
    
    /// Last update date
    pub last_updated: Option<DateTime<Utc>>,
    
    /// Survey methodology notes
    pub methodology: Option<String>,
    
    /// Data collection method
    pub collection_method: Option<String>,
    
    /// Sample size information
    pub sample_size: Option<String>,
    
    /// Coverage information
    pub coverage: Option<String>,
    
    /// Reference period information
    pub reference_period: Option<String>,
    
    /// Publication schedule
    pub publication_schedule: Option<String>,
    
    /// Related surveys
    #[serde(default)]
    pub related_surveys: Vec<String>,
    
    /// Survey-specific settings
    #[serde(default)]
    pub settings: HashMap<String, String>,
    
    /// Custom attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl SurveyMetadata {
    /// Create a new metadata builder
    pub fn builder() -> SurveyMetadataBuilder {
        SurveyMetadataBuilder::new()
    }

    /// Get the effective end date (end_date or current time if active)
    pub fn effective_end_date(&self) -> DateTime<Utc> {
        self.end_date.unwrap_or_else(Utc::now)
    }

    /// Check if the survey is currently active
    pub fn is_currently_active(&self) -> bool {
        self.status == DataStatus::Active && 
        self.end_date.map_or(true, |end| end > Utc::now())
    }

    /// Add a related survey
    pub fn add_related_survey(&mut self, survey_code: &str) {
        let code = survey_code.to_uppercase();
        if !self.related_surveys.contains(&code) {
            self.related_surveys.push(code);
        }
    }

    /// Remove a related survey
    pub fn remove_related_survey(&mut self, survey_code: &str) {
        let code = survey_code.to_uppercase();
        self.related_surveys.retain(|s| s != &code);
    }

    /// Add a setting
    pub fn add_setting(&mut self, key: &str, value: &str) {
        self.settings.insert(key.to_string(), value.to_string());
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }
}

impl Default for SurveyMetadata {
    fn default() -> Self {
        Self {
            description: None,
            status: DataStatus::Active,
            frequency: Frequency::Monthly,
            contact: None,
            start_date: None,
            end_date: None,
            last_updated: None,
            methodology: None,
            collection_method: None,
            sample_size: None,
            coverage: None,
            reference_period: None,
            publication_schedule: None,
            related_surveys: Vec::new(),
            settings: HashMap::new(),
            attributes: HashMap::new(),
        }
    }
}

/// Survey contact information
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SurveyContact {
    /// Contact name
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    /// Contact title/position
    pub title: Option<String>,
    
    /// Contact email
    #[validate(email)]
    pub email: String,
    
    /// Contact phone number
    pub phone: Option<String>,
    
    /// Organization/department
    pub organization: Option<String>,
    
    /// Mailing address
    pub address: Option<String>,
}

impl SurveyContact {
    /// Create a new survey contact
    pub fn new(name: &str, email: &str) -> Self {
        Self {
            name: name.to_string(),
            title: None,
            email: email.to_string(),
            phone: None,
            organization: None,
            address: None,
        }
    }

    /// Create a contact builder
    pub fn builder(name: &str, email: &str) -> SurveyContactBuilder {
        SurveyContactBuilder::new(name, email)
    }
}

/// Builder for SurveyContact
pub struct SurveyContactBuilder {
    name: String,
    email: String,
    title: Option<String>,
    phone: Option<String>,
    organization: Option<String>,
    address: Option<String>,
}

impl SurveyContactBuilder {
    /// Create a new builder
    pub fn new(name: &str, email: &str) -> Self {
        Self {
            name: name.to_string(),
            email: email.to_string(),
            title: None,
            phone: None,
            organization: None,
            address: None,
        }
    }

    /// Set the title
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Set the phone number
    pub fn phone(mut self, phone: &str) -> Self {
        self.phone = Some(phone.to_string());
        self
    }

    /// Set the organization
    pub fn organization(mut self, organization: &str) -> Self {
        self.organization = Some(organization.to_string());
        self
    }

    /// Set the address
    pub fn address(mut self, address: &str) -> Self {
        self.address = Some(address.to_string());
        self
    }

    /// Build the contact
    pub fn build(self) -> SurveyContact {
        SurveyContact {
            name: self.name,
            title: self.title,
            email: self.email,
            phone: self.phone,
            organization: self.organization,
            address: self.address,
        }
    }
}

/// Builder for SurveyMetadata
pub struct SurveyMetadataBuilder {
    metadata: SurveyMetadata,
}

impl SurveyMetadataBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            metadata: SurveyMetadata::default(),
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.metadata.description = Some(description.to_string());
        self
    }

    /// Set the status
    pub fn status(mut self, status: DataStatus) -> Self {
        self.metadata.status = status;
        self
    }

    /// Set the frequency
    pub fn frequency(mut self, frequency: Frequency) -> Self {
        self.metadata.frequency = frequency;
        self
    }

    /// Set the contact
    pub fn contact(mut self, contact: SurveyContact) -> Self {
        self.metadata.contact = Some(contact);
        self
    }

    /// Set the start date
    pub fn start_date(mut self, start_date: DateTime<Utc>) -> Self {
        self.metadata.start_date = Some(start_date);
        self
    }

    /// Set the end date
    pub fn end_date(mut self, end_date: DateTime<Utc>) -> Self {
        self.metadata.end_date = Some(end_date);
        self
    }

    /// Set the last updated date
    pub fn last_updated(mut self, last_updated: DateTime<Utc>) -> Self {
        self.metadata.last_updated = Some(last_updated);
        self
    }

    /// Set the methodology
    pub fn methodology(mut self, methodology: &str) -> Self {
        self.metadata.methodology = Some(methodology.to_string());
        self
    }

    /// Set the collection method
    pub fn collection_method(mut self, collection_method: &str) -> Self {
        self.metadata.collection_method = Some(collection_method.to_string());
        self
    }

    /// Set the sample size
    pub fn sample_size(mut self, sample_size: &str) -> Self {
        self.metadata.sample_size = Some(sample_size.to_string());
        self
    }

    /// Set the coverage
    pub fn coverage(mut self, coverage: &str) -> Self {
        self.metadata.coverage = Some(coverage.to_string());
        self
    }

    /// Set the reference period
    pub fn reference_period(mut self, reference_period: &str) -> Self {
        self.metadata.reference_period = Some(reference_period.to_string());
        self
    }

    /// Set the publication schedule
    pub fn publication_schedule(mut self, publication_schedule: &str) -> Self {
        self.metadata.publication_schedule = Some(publication_schedule.to_string());
        self
    }

    /// Add a related survey
    pub fn related_survey(mut self, survey_code: &str) -> Self {
        self.metadata.add_related_survey(survey_code);
        self
    }

    /// Add a setting
    pub fn setting(mut self, key: &str, value: &str) -> Self {
        self.metadata.add_setting(key, value);
        self
    }

    /// Add an attribute
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.metadata.add_attribute(key, value);
        self
    }

    /// Build the metadata
    pub fn build(self) -> SurveyMetadata {
        self.metadata
    }
}

impl Default for SurveyMetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Custom validation functions
fn validate_survey_code(survey_code: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_survey_code(survey_code)
        .map_err(|_| validator::ValidationError::new("invalid_survey_code"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_survey_new() {
        let survey = Survey::new("ap", "Average Price Data");
        assert_eq!(survey.survey_code, "AP");
        assert_eq!(survey.name, "Average Price Data");
        assert!(survey.is_active());
    }

    #[test]
    fn test_survey_with_metadata() {
        let metadata = SurveyMetadata::builder()
            .description("Consumer price data for selected items")
            .frequency(Frequency::Monthly)
            .status(DataStatus::Active)
            .build();

        let survey = Survey::with_metadata("ap", "Average Price Data", metadata);
        assert_eq!(survey.description(), Some("Consumer price data for selected items"));
        assert_eq!(survey.frequency(), &Frequency::Monthly);
        assert!(survey.is_active());
    }

    #[test]
    fn test_survey_metadata_builder() {
        let contact = SurveyContact::builder("John Doe", "john.doe@bls.gov")
            .title("Survey Manager")
            .phone("202-555-0123")
            .organization("Bureau of Labor Statistics")
            .build();

        let metadata = SurveyMetadata::builder()
            .description("Test survey description")
            .status(DataStatus::Active)
            .frequency(Frequency::Quarterly)
            .contact(contact)
            .methodology("Sample-based survey")
            .collection_method("Electronic data collection")
            .sample_size("10,000 establishments")
            .coverage("All industries")
            .reference_period("Calendar quarter")
            .publication_schedule("45 days after reference period")
            .related_survey("BD")
            .related_survey("CE")
            .setting("max_retries", "3")
            .attribute("priority", "high")
            .build();

        assert_eq!(metadata.description, Some("Test survey description".to_string()));
        assert_eq!(metadata.status, DataStatus::Active);
        assert_eq!(metadata.frequency, Frequency::Quarterly);
        assert!(metadata.contact.is_some());
        assert_eq!(metadata.methodology, Some("Sample-based survey".to_string()));
        assert_eq!(metadata.collection_method, Some("Electronic data collection".to_string()));
        assert_eq!(metadata.sample_size, Some("10,000 establishments".to_string()));
        assert_eq!(metadata.coverage, Some("All industries".to_string()));
        assert_eq!(metadata.reference_period, Some("Calendar quarter".to_string()));
        assert_eq!(metadata.publication_schedule, Some("45 days after reference period".to_string()));
        assert_eq!(metadata.related_surveys, vec!["BD", "CE"]);
        assert_eq!(metadata.settings.get("max_retries"), Some(&"3".to_string()));
        assert_eq!(metadata.attributes.get("priority"), Some(&"high".to_string()));
    }

    #[test]
    fn test_survey_contact_builder() {
        let contact = SurveyContact::builder("Jane Smith", "jane.smith@bls.gov")
            .title("Senior Economist")
            .phone("202-555-0456")
            .organization("Bureau of Labor Statistics")
            .address("2 Massachusetts Ave NE, Washington, DC 20212")
            .build();

        assert_eq!(contact.name, "Jane Smith");
        assert_eq!(contact.email, "jane.smith@bls.gov");
        assert_eq!(contact.title, Some("Senior Economist".to_string()));
        assert_eq!(contact.phone, Some("202-555-0456".to_string()));
        assert_eq!(contact.organization, Some("Bureau of Labor Statistics".to_string()));
        assert_eq!(contact.address, Some("2 Massachusetts Ave NE, Washington, DC 20212".to_string()));
    }

    #[test]
    fn test_survey_metadata_operations() {
        let mut metadata = SurveyMetadata::default();
        
        // Test related surveys
        metadata.add_related_survey("bd");
        metadata.add_related_survey("ce");
        metadata.add_related_survey("BD"); // Duplicate should not be added
        assert_eq!(metadata.related_surveys, vec!["BD", "CE"]);
        
        metadata.remove_related_survey("bd");
        assert_eq!(metadata.related_surveys, vec!["CE"]);
        
        // Test settings and attributes
        metadata.add_setting("timeout", "30");
        metadata.add_attribute("category", "economic");
        
        assert_eq!(metadata.settings.get("timeout"), Some(&"30".to_string()));
        assert_eq!(metadata.attributes.get("category"), Some(&"economic".to_string()));
    }

    #[test]
    fn test_survey_updates() {
        let mut survey = Survey::new("ap", "Average Price Data");
        let original_version = survey.common.version;

        survey.set_description("Updated description");
        assert_eq!(survey.description(), Some("Updated description"));
        assert!(survey.common.version > original_version);

        let new_version = survey.common.version;
        survey.set_status(DataStatus::Discontinued);
        assert!(!survey.is_active());
        assert!(survey.common.version > new_version);
    }

    #[test]
    fn test_survey_validation() {
        let survey = Survey::new("AP", "Valid Survey");
        assert!(survey.validate_survey().is_ok());

        let invalid_survey = Survey::new("X", "Invalid Survey"); // Too short survey code
        assert!(invalid_survey.validate_survey().is_err());
    }

    #[test]
    fn test_survey_metadata_status_checks() {
        let mut metadata = SurveyMetadata::default();
        assert!(metadata.is_currently_active());

        // Set end date in the past
        metadata.end_date = Some(Utc::now() - chrono::Duration::days(30));
        assert!(!metadata.is_currently_active());

        // Set end date in the future
        metadata.end_date = Some(Utc::now() + chrono::Duration::days(30));
        assert!(metadata.is_currently_active());

        // Set status to inactive
        metadata.status = DataStatus::Inactive;
        assert!(!metadata.is_currently_active());
    }

    #[test]
    fn test_effective_end_date() {
        let metadata = SurveyMetadata::default();
        let end_date = metadata.effective_end_date();
        // Should be approximately current time since no end_date is set
        assert!((Utc::now() - end_date).num_seconds().abs() < 5);

        let mut metadata_with_end = SurveyMetadata::default();
        let specific_end = Utc::now() - chrono::Duration::days(30);
        metadata_with_end.end_date = Some(specific_end);
        assert_eq!(metadata_with_end.effective_end_date(), specific_end);
    }

    #[test]
    fn test_serialization() {
        let survey = Survey::new("AP", "Average Price Data");
        let serialized = serde_json::to_string(&survey).unwrap();
        let deserialized: Survey = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(survey.survey_code, deserialized.survey_code);
        assert_eq!(survey.name, deserialized.name);
        assert_eq!(survey.is_active(), deserialized.is_active());
    }
}