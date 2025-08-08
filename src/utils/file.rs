use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use memmap2::MmapOptions;
use serde_yaml;

use crate::error::types::{Error as CrateError, SystemError};
use crate::utils::path::PathUtils;

// Assuming Result is type aliased to crate::error::types::Error somewhere:
// type Result<T> = std::result::Result<T, CrateError>;
type Result<T> = std::result::Result<T, CrateError>;

// Assuming there's an ensure_directory utility that returns Result<()>
pub(crate) use crate::utils::path::ensure_directory;

pub struct FileUtils {
    path_utils: PathUtils,
}

impl FileUtils {
    /// Create a new FileUtils instance
    pub fn new() -> Self {
        Self {
            path_utils: PathUtils::new(),
        }
    }

    /// Create FileUtils with a specific path utils instance
    pub fn with_path_utils(path_utils: PathUtils) -> Self {
        Self { path_utils }
    }

    /// Get file size in bytes
    pub fn get_file_size<P: AsRef<Path>>(&self, path: P) -> Result<u64> {
        let path = path.as_ref();
        self.path_utils.validate_path(path)?;

        let metadata = fs::metadata(path).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("fs::metadata({})", path.display()),
                source: e.to_string(),
            })
        })?;

        Ok(metadata.len())
    }

    /// Check if file exists
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists() && path.as_ref().is_file()
    }

    /// Check if directory exists
    pub fn directory_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref().exists() && path.as_ref().is_dir()
    }

    /// Get file extension
    pub fn get_file_extension<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Create a backup of a file
    pub fn backup_file<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        self.path_utils.validate_path(path)?;

        if !self.file_exists(path) {
            return Err(CrateError::System(SystemError::ResourceError {
                resource: "file".to_string(),
                message: format!("Not found: {}", path.display()),
            }));
        }

        let backup_path = path.with_extension(format!(
            "{}.backup",
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
        ));

        fs::copy(path, &backup_path).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("fs::copy({}, {})", path.display(), backup_path.display()),
                source: e.to_string(),
            })
        })?;

        Ok(backup_path)
    }

    /// Atomic write operation (write to temp file, then rename)
    pub fn atomic_write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, content: C) -> Result<()> {
        let path = path.as_ref();
        self.path_utils.validate_path(path)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            ensure_directory(parent)?;
        }

        // Create temporary file in same directory
        let temp_path = path.with_extension("tmp");

        // Write to temporary file
        {
            let mut file = File::create(&temp_path).map_err(|e| {
                CrateError::System(SystemError::IoError {
                    operation: format!("File::create({})", temp_path.display()),
                    source: e.to_string(),
                })
            })?;
            file.write_all(content.as_ref()).map_err(|e| {
                CrateError::System(SystemError::IoError {
                    operation: format!("write_all({})", temp_path.display()),
                    source: e.to_string(),
                })
            })?;
            file.sync_all().map_err(|e| {
                CrateError::System(SystemError::IoError {
                    operation: format!("sync_all({})", temp_path.display()),
                    source: e.to_string(),
                })
            })?;
        }

        // Atomically rename temporary file to target
        fs::rename(&temp_path, path).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("fs::rename({}, {})", temp_path.display(), path.display()),
                source: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Create memory-mapped file for large file operations
    pub fn create_mmap<P: AsRef<Path>>(&self, path: P) -> Result<memmap2::Mmap> {
        let path = path.as_ref();
        self.path_utils.validate_path(path)?;

        let file = File::open(path).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("File::open({})", path.display()),
                source: e.to_string(),
            })
        })?;

        unsafe {
            MmapOptions::new().map(&file).map_err(|e| {
                CrateError::System(SystemError::IoError {
                    operation: "MmapOptions::map(&file)".to_string(),
                    source: e.to_string(),
                })
            })
        }
    }

    /// Get file modification time as Unix timestamp
    pub fn get_modification_time<P: AsRef<Path>>(&self, path: P) -> Result<u64> {
        let path = path.as_ref();
        self.path_utils.validate_path(path)?;

        let metadata = fs::metadata(path).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("fs::metadata({})", path.display()),
                source: e.to_string(),
            })
        })?;

        let modified = metadata.modified().map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("Metadata::modified({})", path.display()),
                source: e.to_string(),
            })
        })?;

        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                CrateError::System(SystemError::TimeError {
                    operation: "SystemTime::duration_since(UNIX_EPOCH)".to_string(),
                    message: e.to_string(),
                })
            })?;

        Ok(duration.as_secs())
    }

    /// List files in directory with optional extension filter
    pub fn list_files<P: AsRef<Path>>(&self, dir: P, extension: Option<&str>) -> Result<Vec<PathBuf>> {
        let dir = dir.as_ref();
        self.path_utils.validate_path(dir)?;

        if !self.directory_exists(dir) {
            return Err(CrateError::System(SystemError::ResourceError {
                resource: "directory".to_string(),
                message: format!("Not found: {}", dir.display()),
            }));
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            CrateError::System(SystemError::IoError {
                operation: format!("fs::read_dir({})", dir.display()),
                source: e.to_string(),
            })
        })?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                CrateError::System(SystemError::IoError {
                    operation: format!("DirEntry iteration in {}", dir.display()),
                    source: e.to_string(),
                })
            })?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = extension {
                    if let Some(file_ext) = self.get_file_extension(&path) {
                        if file_ext == ext.to_lowercase() {
                            files.push(path);
                        }
                    }
                } else {
                    files.push(path);
                }
            }
        }

        files.sort();
        Ok(files)
    }
}

// Standalone convenience functions for common file operations
/// Get file size in bytes
pub fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let file_utils = FileUtils::new();
    file_utils.get_file_size(path)
}

/// Check if file is readable (exists and is a file)
pub fn is_file_readable<P: AsRef<Path>>(path: P) -> bool {
    let file_utils = FileUtils::new();
    file_utils.file_exists(path)
}

/// Read file contents as string
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let file_utils = FileUtils::new();
    let path = path.as_ref();
    file_utils.path_utils.validate_path(path)?;
    
    fs::read_to_string(path).map_err(|e| {
        CrateError::System(SystemError::IoError {
            operation: format!("fs::read_to_string({})", path.display()),
            source: e.to_string(),
        })
    })
}

/// Write string content to file
pub fn write_string<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let file_utils = FileUtils::new();
    let path = path.as_ref();
    file_utils.path_utils.validate_path(path)?;
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        ensure_directory(parent)?;
    }
    
    fs::write(path, content).map_err(|e| {
        CrateError::System(SystemError::IoError {
            operation: format!("fs::write({})", path.display()),
            source: e.to_string(),
        })
    })
}

/// Read and parse YAML file
pub fn read_yaml<T, P>(path: P) -> Result<T>
where
    T: serde::de::DeserializeOwned,
    P: AsRef<Path>,
{
    let file_utils = FileUtils::new();
    let path = path.as_ref();
    file_utils.path_utils.validate_path(path)?;
    
    let content = fs::read_to_string(path).map_err(|e| {
        CrateError::System(SystemError::IoError {
            operation: format!("fs::read_to_string({})", path.display()),
            source: e.to_string(),
        })
    })?;
    
    serde_yaml::from_str(&content).map_err(|e| {
        CrateError::System(SystemError::ParseError {
            format: "YAML".to_string(),
            source: e.to_string(),
            context: Some(format!("file: {}", path.display())),
        })
    })
}

