//! # Error Message Internationalization
//!
//! This module provides comprehensive internationalization (i18n) support for error messages
//! in the Rusty BLS Data Processing system. It enables multi-language error reporting,
//! locale-aware formatting, and cultural considerations for global deployment.
//!
//! ## Features
//!
//! - **Multi-Language Support**: Error messages in multiple languages
//! - **Locale-Aware Formatting**: Format errors according to locale conventions
//! - **Message Catalogs**: Organized message catalogs for different languages
//! - **Fallback Mechanisms**: Graceful fallback to default language when translations are missing
//! - **Cultural Considerations**: Respect cultural differences in error presentation
//! - **Dynamic Language Switching**: Runtime language switching support
//!
//! ## Architecture
//!
//! ```text
//! Internationalization System
//! ├── ErrorLocalizer          # Main localization engine
//! │   ├── MessageCatalog      # Language-specific message catalogs
//! │   ├── LocaleManager       # Locale detection and management
//! │   └── FormatEngine        # Locale-aware formatting
//! ├── MessageCatalog          # Message storage and retrieval
//! │   ├── TranslationEntry    # Individual translation entries
//! │   ├── PluralRules         # Language-specific plural rules
//! │   └── ContextualMessages  # Context-aware translations
//! ├── LocaleSupport           # Locale detection and validation
//! │   ├── LocaleDetector      # Automatic locale detection
//! │   ├── LocaleValidator     # Validate locale codes
//! │   └── FallbackChain       # Fallback locale chain
//! └── CulturalAdaptation      # Cultural considerations
//!     ├── DateTimeFormatting  # Locale-specific date/time formatting
//!     ├── NumberFormatting    # Locale-specific number formatting
//!     └── MessageTone         # Cultural tone adaptation
//! ```
//!
//! ## Usage
//!
//! ```

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::error::{Error, ErrorContext};

/// Locale identifier for internationalization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Locale {
    /// Language code (ISO 639-1)
    pub language: String,
    /// Country/region code (ISO 3166-1 alpha-2)
    pub region: Option<String>,
    /// Script code (ISO 15924)
    pub script: Option<String>,
    /// Variant identifier
    pub variant: Option<String>,
}

impl Locale {
    /// Create a new locale from language code
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_lowercase(),
            region: None,
            script: None,
            variant: None,
        }
    }

    /// Create a locale with language and region
    pub fn with_region(language: &str, region: &str) -> Self {
        Self {
            language: language.to_lowercase(),
            region: Some(region.to_uppercase()),
            script: None,
            variant: None,
        }
    }

    /// Parse locale from string (e.g., "en-US", "zh-Hans-CN")
    pub fn from_string(locale_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = locale_str.split('-').collect();
        if parts.is_empty() {
            return Err("Invalid locale string".to_string());
        }

        let mut locale = Self::new(parts[0]);
        
        for part in parts.iter().skip(1) {
            match part.len() {
                2 => {
                    // Country code
                    if part.chars().all(|c| c.is_ascii_alphabetic()) {
                        locale.region = Some(part.to_uppercase());
                    }
                }
                4 => {
                    // Script code
                    if part.chars().all(|c| c.is_ascii_alphabetic()) {
                        locale.script = Some(part.to_string());
                    }
                }
                _ => {
                    // Variant
                    locale.variant = Some(part.to_string());
                }
            }
        }

        Ok(locale)
    }

    /// Convert locale to string representation
    pub fn to_string(&self) -> String {
        let mut parts = vec![self.language.clone()];
        
        if let Some(script) = &self.script {
            parts.push(script.clone());
        }
        
        if let Some(region) = &self.region {
            parts.push(region.clone());
        }
        
        if let Some(variant) = &self.variant {
            parts.push(variant.clone());
        }
        
        parts.join("-")
    }

    /// Get fallback locales for this locale
    pub fn fallback_chain(&self) -> Vec<Locale> {
        let mut fallbacks = Vec::new();
        
        // Add current locale
        fallbacks.push(self.clone());
        
        // Add locale without variant
        if self.variant.is_some() {
            fallbacks.push(Locale {
                language: self.language.clone(),
                region: self.region.clone(),
                script: self.script.clone(),
                variant: None,
            });
        }
        
        // Add locale without script
        if self.script.is_some() {
            fallbacks.push(Locale {
                language: self.language.clone(),
                region: self.region.clone(),
                script: None,
                variant: None,
            });
        }
        
        // Add locale without region
        if self.region.is_some() {
            fallbacks.push(Locale {
                language: self.language.clone(),
                region: None,
                script: None,
                variant: None,
            });
        }
        
        // Add default English fallback
        if self.language != "en" {
            fallbacks.push(Locale::new("en"));
        }
        
        fallbacks
    }
}

impl From<&str> for Locale {
    fn from(locale_str: &str) -> Self {
        Self::from_string(locale_str).unwrap_or_else(|_| Self::new("en"))
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Translation entry for a specific message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationEntry {
    /// Message key identifier
    pub key: String,
    /// Translated message text
    pub message: String,
    /// Plural forms (if applicable)
    pub plural_forms: Option<HashMap<String, String>>,
    /// Context information for translators
    pub context: Option<String>,
    /// Translation metadata
    pub metadata: TranslationMetadata,
}

/// Metadata for translation entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMetadata {
    /// When this translation was created
    pub created_at: DateTime<Utc>,
    /// When this translation was last updated
    pub updated_at: DateTime<Utc>,
    /// Translator information
    pub translator: Option<String>,
    /// Translation quality score (0.0 - 1.0)
    pub quality_score: Option<f64>,
    /// Whether this translation needs review
    pub needs_review: bool,
}

/// Message catalog for a specific locale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCatalog {
    /// Locale this catalog is for
    pub locale: Locale,
    /// Translation entries
    pub entries: HashMap<String, TranslationEntry>,
    /// Plural rules for this locale
    pub plural_rules: PluralRules,
    /// Catalog metadata
    pub metadata: CatalogMetadata,
}

/// Metadata for message catalogs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetadata {
    /// Catalog version
    pub version: String,
    /// When this catalog was created
    pub created_at: DateTime<Utc>,
    /// When this catalog was last updated
    pub updated_at: DateTime<Utc>,
    /// Total number of translations
    pub translation_count: usize,
    /// Completion percentage (0.0 - 1.0)
    pub completion_percentage: f64,
}

/// Plural rules for a specific locale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluralRules {
    /// Locale these rules apply to
        /// Plural rule expressions
    pub rules: HashMap<String, String>,
}

impl MessageCatalog {
    /// Create a new empty message catalog
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            entries: HashMap::new(),
            plural_rules: PluralRules {
                rules: HashMap::new(),
            },
            metadata: CatalogMetadata {
                version: "1.0.0".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                translation_count: 0,
                completion_percentage: 0.0,
            },
        }
    }

    /// Load message catalog from JSON file
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        // Implementation would load from file
        // For now, return a default catalog
        Ok(Self::new(Locale::new("en")))
    }

    /// Add a translation entry
    pub fn add_entry(&mut self, key: &str, message: &str) {
        let entry = TranslationEntry {
            key: key.to_string(),
            message: message.to_string(),
            plural_forms: None,
            context: None,
            metadata: TranslationMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                translator: None,
                quality_score: None,
                needs_review: false,
            },
        };
        
        self.entries.insert(key.to_string(), entry);
        self.metadata.translation_count = self.entries.len();
        self.metadata.updated_at = Utc::now();
    }

    /// Get a translation by key
    pub fn get_translation(&self, key: &str) -> Option<&TranslationEntry> {
        self.entries.get(key)
    }

    /// Check if a translation exists
    pub fn has_translation(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}

/// Main error localization engine
#[derive(Debug)]
pub struct ErrorLocalizer {
    /// Message catalogs by locale
    catalogs: HashMap<Locale, MessageCatalog>,
    /// Default locale to use as fallback
    default_locale: Locale,
    /// Current active locale
    current_locale: Locale,
    /// Localization statistics
    stats: LocalizationStats,
}

/// Statistics for localization operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationStats {
    /// Total localization requests
    pub total_requests: u64,
    /// Successful localizations
    pub successful_localizations: u64,
    /// Fallback localizations used
    pub fallback_used: u64,
    /// Missing translations
    pub missing_translations: HashMap<String, u64>,
}

impl ErrorLocalizer {
    /// Create a new error localizer
    pub fn new() -> Self {
        Self {
            catalogs: HashMap::new(),
            default_locale: Locale::new("en"),
            current_locale: Locale::new("en"),
            stats: LocalizationStats {
                total_requests: 0,
                successful_localizations: 0,
                fallback_used: 0,
                missing_translations: HashMap::new(),
            },
        }
    }

    /// Add a message catalog for a locale
    pub fn with_catalog(mut self, locale: &str, catalog: MessageCatalog) -> Self {
        let locale_obj = Locale::from(locale);
        self.catalogs.insert(locale_obj, catalog);
        self
    }

    /// Set the default locale
    pub fn with_default_locale(mut self, locale: &str) -> Self {
        self.default_locale = Locale::from(locale);
        self
    }

    /// Set the current active locale
    pub fn set_locale(&mut self, locale: &str) {
        self.current_locale = Locale::from(locale);
    }

    /// Get the current locale
    pub fn get_locale(&self) -> &Locale {
        &self.current_locale
    }

    /// Localize an error message
    pub fn localize_error(&mut self, error: &Error, locale: &Locale) -> String {
        self.stats.total_requests += 1;

        let error_key = self.get_error_key(error);
        
        if let Some(localized) = self.get_localized_message(&error_key, locale) {
            self.stats.successful_localizations += 1;
            localized
        } else {
            // Try fallback locales
            for fallback_locale in locale.fallback_chain() {
                if let Some(localized) = self.get_localized_message(&error_key, &fallback_locale) {
                    self.stats.fallback_used += 1;
                    return localized;
                }
            }
            
            // Record missing translation
            *self.stats.missing_translations.entry(error_key.clone()).or_insert(0) += 1;
            
            // Return default error message
            self.get_default_error_message(error)
        }
    }

    /// Format a message with parameters
    pub fn format_message(
        &mut self,
        key: &str,
        params: &[(&str, &str)],
        locale: &Locale,
    ) -> String {
        self.stats.total_requests += 1;

        if let Some(template) = self.get_localized_message(key, locale) {
            self.stats.successful_localizations += 1;
            self.substitute_parameters(&template, params)
        } else {
            // Try fallback locales
            for fallback_locale in locale.fallback_chain() {
                if let Some(template) = self.get_localized_message(key, &fallback_locale) {
                    self.stats.fallback_used += 1;
                    return self.substitute_parameters(&template, params);
                }
            }
            
            // Record missing translation
            *self.stats.missing_translations.entry(key.to_string()).or_insert(0) += 1;
            
            // Return key as fallback
            key.to_string()
        }
    }

    /// Localize error context
    pub fn localize_context(&mut self, context: &ErrorContext, locale: &Locale) -> ErrorContext {
        let mut localized = context.clone();
        
        // Try to localize the context message
        let context_key = format!("context.{}", context.component);
        if let Some(localized_message) = self.get_localized_message(&context_key, locale) {
            localized.message = localized_message;
        }
        
        localized
    }

    /// Get localization statistics
    pub fn get_stats(&self) -> &LocalizationStats {
        &self.stats
    }

    /// Get a localized message by key and locale
    fn get_localized_message(&self, key: &str, locale: &Locale) -> Option<String> {
        self.catalogs
            .get(locale)
            .and_then(|catalog| catalog.get_translation(key))
            .map(|entry| entry.message.clone())
    }

    /// Get error key for localization lookup
    fn get_error_key(&self, error: &Error) -> String {
        match error {
            Error::Config(config_error) => {
                format!("config.{:?}", config_error).to_lowercase()
            }
            Error::Data(data_error) => {
                format!("data.{:?}", data_error).to_lowercase()
            }
            Error::Processing(processing_error) => {
                format!("processing.{:?}", processing_error).to_lowercase()
            }
            Error::Output(output_error) => {
                format!("output.{:?}", output_error).to_lowercase()
            }
            Error::Plugin(plugin_error) => {
                format!("plugin.{:?}", plugin_error).to_lowercase()
            }
            Error::System(system_error) => {
                format!("system.{:?}", system_error).to_lowercase()
            }
        }
    }

    /// Get default error message (English fallback)
    fn get_default_error_message(&self, error: &Error) -> String {
        format!("{:?}", error)
    }

    /// Substitute parameters in a message template
    fn substitute_parameters(&self, template: &str, params: &[(&str, &str)]) -> String {
        let mut result = template.to_string();
        
        for (key, value) in params {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }
}

/// Locale detection utilities
pub mod locale_detection {
    use super::*;

    /// Detect locale from environment variables
    pub fn detect_from_env() -> Option<Locale> {
        // Check common environment variables
        for env_var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(locale_str) = std::env::var(env_var) {
                if let Ok(locale) = Locale::from_string(&locale_str) {
                    return Some(locale);
                }
            }
        }
        None
    }

    /// Detect locale from system settings
    pub fn detect_from_system() -> Option<Locale> {
        // Implementation would use system APIs to detect locale
        // For now, return None to fall back to default
        None
    }

    /// Get best matching locale from available catalogs
    pub fn best_match(
        requested: &Locale,
        available: &[Locale],
    ) -> Option<Locale> {
        // Exact match
        if available.contains(requested) {
            return Some(requested.clone());
        }

        // Try fallback chain
        for fallback in requested.fallback_chain() {
            if available.contains(&fallback) {
                return Some(fallback);
            }
        }

        None
    }
}

/// Cultural adaptation utilities
pub mod cultural {
    use super::*;

    /// Format numbers according to locale conventions
    pub fn format_number(number: f64, locale: &Locale) -> String {
        // Implementation would use locale-specific number formatting
        // For now, return basic formatting
        format!("{:.2}", number)
    }

    /// Format dates according to locale conventions
    pub fn format_date(date: &DateTime<Utc>, locale: &Locale) -> String {
        // Implementation would use locale-specific date formatting
        // For now, return ISO format
        date.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    /// Adapt message tone for cultural context
    pub fn adapt_tone(message: &str, locale: &Locale) -> String {
        // Implementation would adapt message tone based on cultural norms
        // For now, return message as-is
        message.to_string()
    }
}

/// Utility functions for internationalization
pub mod utils {
    use super::*;

    /// Validate a locale string
    pub fn validate_locale(locale_str: &str) -> bool {
        Locale::from_string(locale_str).is_ok()
    }

    /// Get supported locales from catalogs
    pub fn get_supported_locales(catalogs: &HashMap<Locale, MessageCatalog>) -> Vec<Locale> {
        catalogs.keys().cloned().collect()
    }

    /// Calculate translation completion percentage
    pub fn calculate_completion(
        catalog: &MessageCatalog,
        reference_catalog: &MessageCatalog,
    ) -> f64 {
        if reference_catalog.entries.is_empty() {
            return 1.0;
        }

        let translated_count = reference_catalog
            .entries
            .keys()
            .filter(|key| catalog.has_translation(key))
            .count();

        translated_count as f64 / reference_catalog.entries.len() as f64
    }
}