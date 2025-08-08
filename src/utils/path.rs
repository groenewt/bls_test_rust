//! # Path Utilities
//!
//! This module provides path manipulation and validation utilities for the Rusty BLS Data Processing system.
//! It handles standard path operations, directory structure management, and path validation.
//!
//! ## Features
//!
//! - Standard path construction for config, data, and output directories
//! - Path validation and sanitization
//! - Cross-platform path handling
//! - Security checks to prevent path traversal attacks
//!
//! ## Usage


use std::path::{Path, PathBuf};
use std::env;
use crate::error::{Result, SystemError};

/// Path utilities for the BLS data processing system
pub struct PathUtils {
    project_root: PathBuf,
}

impl PathUtils {
    /// Create a new PathUtils instance
    pub fn new() -> Self {
        let project_root = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        
        Self { project_root }
    }

    /// Create PathUtils with a specific project root
    pub fn with_root<P: AsRef<Path>>(root: P) -> Self {
        Self {
            project_root: root.as_ref().to_path_buf(),
        }
    }

    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Validate a path for security and correctness
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Check for path traversal attempts
        if path.to_string_lossy().contains("..") {
            return Err(crate::error::Error::System(SystemError::invalid_path(
                format!("Path contains invalid traversal: {}", path.display())
            )));
        }

        // Check for absolute paths outside project root (security measure)
        if path.is_absolute() {
            let canonical_path = path.canonicalize()
                .map_err(|e| crate::error::Error::System(SystemError::IoError {
                    operation: "canonicalize path".to_string(),
                    source: e.to_string(),
                }))?;
            let canonical_root = self.project_root.canonicalize()
                .map_err(|e| crate::error::Error::System(SystemError::IoError {
                    operation: "canonicalize project root".to_string(),
                    source: e.to_string(),
                }))?;
            
            if !canonical_path.starts_with(canonical_root) {
                return Err(crate::error::Error::System(SystemError::InvalidPath(
                    format!("Path is outside project root: {}", path.display())
                )));
            }
        }

        Ok(())
    }

    /// Sanitize a path component (remove invalid characters)
    pub fn sanitize_component(&self, component: &str) -> String {
        component
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
            .collect()
    }

    /// Build a path relative to project root
    pub fn build_path<I, P>(&self, components: I) -> PathBuf
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut path = self.project_root.clone();
        for component in components {
            path.push(component);
        }
        path
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> PathBuf {
        self.build_path(&["config"])
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> PathBuf {
        self.build_path(&["data"])
    }

    /// Get the output directory path
    pub fn output_dir(&self) -> PathBuf {
        self.build_path(&["data", "processed"])
    }

    /// Get the final output directory path
    pub fn final_output_dir(&self) -> PathBuf {
        self.build_path(&["data", "final"])
    }
}

impl Default for PathUtils {
    fn default() -> Self {
        Self::new()
    }
}

/// Get a configuration file path
pub fn get_config_path<P1: AsRef<Path>, P2: AsRef<Path>>(subdir: P1, filename: P2) -> Result<PathBuf> {
    let path_utils = PathUtils::new();
    let mut path = path_utils.build_path(&["config"]);
    path.push(subdir.as_ref());
    path.push(filename.as_ref());
    path_utils.validate_path(&path)?;
    Ok(path)
}

/// Get a data file path
pub fn get_data_path<P1: AsRef<Path>, P2: AsRef<Path>, P3: AsRef<Path>>(
    data_type: P1, 
    source: P2, 
    survey: P3
) -> Result<PathBuf> {
    let path_utils = PathUtils::new();
    let mut path = path_utils.build_path(&["data"]);
    path.push(data_type.as_ref());
    path.push(source.as_ref());
    path.push(survey.as_ref());
    path_utils.validate_path(&path)?;
    Ok(path)
}

/// Get an output file path
pub fn get_output_path<P1: AsRef<Path>, P2: AsRef<Path>>(subdir: P1, filename: P2) -> Result<PathBuf> {
    let path_utils = PathUtils::new();
    let mut path = path_utils.build_path(&["data", "processed"]);
    path.push(subdir.as_ref());
    path.push(filename.as_ref());
    path_utils.validate_path(&path)?;
    Ok(path)
}

/// Get a final output file path
pub fn get_final_output_path<P1: AsRef<Path>, P2: AsRef<Path>>(subdir: P1, filename: P2) -> Result<PathBuf> {
    let path_utils = PathUtils::new();
    let mut path = path_utils.build_path(&["data", "final"]);
    path.push(subdir.as_ref());
    path.push(filename.as_ref());
    path_utils.validate_path(&path)?;
    Ok(path)
}

/// Get a temporary file path
pub fn get_temp_path<P: AsRef<Path>>(filename: P) -> Result<PathBuf> {
    let path_utils = PathUtils::new();
    let mut path = path_utils.build_path(&["target", "tmp"]);
    path.push(filename.as_ref());
    path_utils.validate_path(&path)?;
    Ok(path)
}

/// Get survey directory path
pub fn survey_dir(survey_code: &str) -> String {
    let normalized_code = survey_code.to_uppercase();
    format!("surveys/{}", normalized_code)
}

/// Get survey file path
pub fn survey_file(survey_code: &str, filename: &str) -> String {
    let normalized_code = survey_code.to_uppercase();
    format!("surveys/{}/{}", normalized_code, filename)
}

/// Get overrides default file path
pub fn overrides_default(survey_code: &str) -> String {
    let normalized_code = survey_code.to_uppercase();
    format!("surveys/{}/overrides/defaults.yml", normalized_code)
}

/// Get overrides environment file path
pub fn overrides_env(survey_code: &str, env: &str) -> String {
    let normalized_code = survey_code.to_uppercase();
    format!("surveys/{}/overrides/env/{}.yml", normalized_code, env)
}

/// Get overrides local file path
pub fn overrides_local(survey_code: &str) -> String {
    let normalized_code = survey_code.to_uppercase();
    format!("surveys/{}/overrides/local.yml", normalized_code)
}

/// Get shared directory path
pub fn shared_dir() -> String {
    "surveys/_shared".to_string()
}

/// Get shared macro file path
pub fn shared_macro_file(filename: &str) -> String {
    format!("surveys/_shared/macros/{}", filename)
}

/// Get archive directory path
pub fn archive_dir() -> String {
    "config/archive".to_string()
}

/// Normalize survey code to uppercase
pub fn normalize_survey_code(survey_code: &str) -> String {
    survey_code.to_uppercase()
}

/// Validate path against traversal attacks
pub fn validate_path_security(path: &str) -> Result<()> {
    if path.contains("..") || path.contains("~") {
        return Err(crate::error::Error::System(SystemError::IoError {
            operation: "path validation".to_string(),
            source: format!("Path traversal detected: {}", path),
        }));
    }
    Ok(())
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_directory<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| crate::error::Error::System(SystemError::IoError {
                operation: "create directory".to_string(),
                source: e.to_string(),
            }))?;
    } else if !path.is_dir() {
        return Err(crate::error::Error::System(SystemError::InvalidPath(
            format!("Path exists but is not a directory: {}", path.display())
        )));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_path_utils_new() {
        let path_utils = PathUtils::new();
        assert!(path_utils.project_root().exists());
    }

    #[test]
    fn test_path_utils_with_root() {
        let temp_dir = TempDir::new().unwrap();
        let path_utils = PathUtils::with_root(temp_dir.path());
        assert_eq!(path_utils.project_root(), temp_dir.path());
    }

    #[test]
    fn test_validate_path_traversal() {
        let path_utils = PathUtils::new();
        let result = path_utils.validate_path("../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        let path_utils = PathUtils::new();
        let result = path_utils.validate_path("config/surveys/ap.yml");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_component() {
        let path_utils = PathUtils::new();
        let sanitized = path_utils.sanitize_component("test@#$%file.txt");
        assert_eq!(sanitized, "testfile.txt");
    }

    #[test]
    fn test_build_path() {
        let path_utils = PathUtils::new();
        let path = path_utils.build_path(&["config", "surveys", "ap.yml"]);
        assert!(path.to_string_lossy().contains("config"));
        assert!(path.to_string_lossy().contains("surveys"));
        assert!(path.to_string_lossy().contains("ap.yml"));
    }

    #[test]
    fn test_config_dir() {
        let path_utils = PathUtils::new();
        let config_dir = path_utils.config_dir();
        assert!(config_dir.to_string_lossy().ends_with("config"));
    }

    #[test]
    fn test_get_config_path() {
        let result = get_config_path("surveys", "ap.yml");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("config"));
        assert!(path.to_string_lossy().contains("surveys"));
        assert!(path.to_string_lossy().contains("ap.yml"));
    }

    #[test]
    fn test_get_data_path() {
        let result = get_data_path("raw", "bls", "ap");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("data"));
        assert!(path.to_string_lossy().contains("raw"));
        assert!(path.to_string_lossy().contains("bls"));
        assert!(path.to_string_lossy().contains("ap"));
    }

    #[test]
    fn test_ensure_directory() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_dir");
        
        // Directory doesn't exist initially
        assert!(!test_dir.exists());
        
        // Create directory
        let result = ensure_directory(&test_dir);
        assert!(result.is_ok());
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
        
        // Calling again should succeed
        let result = ensure_directory(&test_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_directory_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_file");
        
        // Create a file
        fs::write(&test_file, "test").unwrap();
        
        // Try to create directory with same name should fail
        let result = ensure_directory(&test_file);
        assert!(result.is_err());
    }
}