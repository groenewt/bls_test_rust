//! Data reader module for BLS data processing
//!
//! This module provides comprehensive reading capabilities for BLS data files
//! with support for different file formats, reading strategies, and performance
//! optimizations.

pub mod factory;
pub mod file_reader;
pub mod mmap_reader;
pub mod traits;

// Re-export public types and traits
pub use traits::{
    DataReader, LookupReader, MemoryMappedReader, ObservationReader, ReadStats, ReaderConfig,
    ReaderFactory, SeriesReader, StreamingReader, SurveyReader,
};

pub use factory::{DefaultReaderFactory, ReaderFactoryConfig, ReaderFactoryConfigBuilder};
pub use file_reader::FileReader;
pub use mmap_reader::MmapReader;

/// Create a default reader factory
pub fn create_default_factory() -> DefaultReaderFactory {
    DefaultReaderFactory::new()
}

/// Create a reader factory with custom configuration
pub fn create_factory_with_config(config: ReaderFactoryConfig) -> DefaultReaderFactory {
    DefaultReaderFactory::with_config(config)
}

/// Convenience function to create an optimized reader for a file
pub fn create_optimized_reader(
    path: &std::path::Path,
) -> crate::error::types::Result<Box<dyn DataReader>> {
    let factory = create_default_factory();
    factory.create_optimized_reader(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_module_exports() {
        // Test that we can create a factory
        let factory = create_default_factory();
        assert!(!factory.get_supported_file_types().is_empty());
    }

    #[test]
    fn test_convenience_functions() {
        let factory = create_default_factory();
        assert!(!factory.get_supported_file_types().is_empty());

        let config = ReaderFactoryConfigBuilder::new()
            .mmap_threshold(1024)
            .build();
        let custom_factory = create_factory_with_config(config);
        assert!(!custom_factory.get_supported_file_types().is_empty());
    }
}
