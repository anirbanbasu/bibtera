//! Configuration management module.
//!
//! This module provides configuration management for the BibTeX converter,
//! including command-line arguments and validation.

use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for configuration operations
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Error when reading configuration file
    #[error("Failed to read configuration file: {0}")]
    Read(String),

    /// Error when parsing configuration file
    #[error("Failed to parse configuration file: {0}")]
    Parse(String),

    /// Error when writing configuration file
    #[error("Failed to write configuration file: {0}")]
    Write(String),

    /// Error when configuration is invalid
    #[error("Invalid configuration: {0}")]
    Validation(String),
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Input BibTeX file path
    #[serde(default)]
    pub input: Option<String>,

    /// Output directory path where generated files will be saved
    #[serde(default)]
    pub output: Option<String>,

    /// Path to Tera template file used for formatting
    #[serde(default)]
    pub template: Option<String>,

    /// List of BibTeX entry keys to exclude from processing
    #[serde(default)]
    pub exclude: Vec<String>,

    /// List of BibTeX entry keys to include in processing (if set, only these entries are processed)
    #[serde(default)]
    pub include: Vec<String>,

    /// Perform a dry run without generating files
    #[serde(default)]
    pub dry_run: bool,

    /// Force overwrite of existing files without prompting
    #[serde(default)]
    pub overwrite: bool,

    /// Enable verbose output
    #[serde(default)]
    pub verbose: bool,
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config from command-line arguments
    #[allow(clippy::too_many_arguments)]
    pub fn from_cli(
        input: Option<String>,
        output: Option<String>,
        template: Option<String>,
        exclude: Option<String>,
        include: Option<String>,
        dry_run: bool,
        overwrite: bool,
        verbose: bool,
    ) -> Result<Self> {
        let exclude: Vec<String> = exclude
            .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
            .unwrap_or_default();

        let include: Vec<String> = include
            .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
            .unwrap_or_default();

        // Validate that exclude and include are mutually exclusive
        if !exclude.is_empty() && !include.is_empty() {
            return Err(ConfigError::Validation(
                "Cannot specify both --exclude and --include at the same time".to_string(),
            )
            .into());
        }

        Ok(Self {
            input,
            output,
            template,
            exclude,
            include,
            dry_run,
            overwrite,
            verbose,
        })
    }

    /// Load config from a JSON file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let content =
            fs::read_to_string(path).context(ConfigError::Read(path.display().to_string()))?;

        let config: Config = serde_json::from_str(&content)
            .context(ConfigError::Parse(path.display().to_string()))?;

        Ok(config)
    }

    /// Save config to a JSON file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        let content = serde_json::to_string_pretty(self)
            .context(ConfigError::Write("Failed to serialize config".to_string()))?;

        fs::write(path, &content).context(ConfigError::Write(path.display().to_string()))?;

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check input is provided
        if self.input.is_none() {
            return Err(
                ConfigError::Validation("Input file is required (--input/-i)".to_string()).into(),
            );
        }

        // Check output is provided
        if self.output.is_none() {
            return Err(ConfigError::Validation(
                "Output directory is required (--output/-o)".to_string(),
            )
            .into());
        }

        // Check template is provided
        if self.template.is_none() {
            return Err(ConfigError::Validation(
                "Template file is required (--template/-t)".to_string(),
            )
            .into());
        }

        // Validate input path exists
        if let Some(input) = &self.input
            && !std::path::Path::new(input).exists()
        {
            return Err(
                ConfigError::Validation(format!("Input path does not exist: {}", input)).into(),
            );
        }

        // Validate template path exists
        if let Some(template) = &self.template
            && !std::path::Path::new(template).exists()
        {
            return Err(ConfigError::Validation(format!(
                "Template path does not exist: {}",
                template
            ))
            .into());
        }

        Ok(())
    }

    /// Check if an entry key should be included
    pub fn should_include_entry(&self, key: &str) -> bool {
        if !self.include.is_empty() {
            return self.include.contains(&key.to_string());
        }

        if !self.exclude.is_empty() {
            return !self.exclude.contains(&key.to_string());
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.verbose, false);
        assert_eq!(config.dry_run, false);
        assert!(config.exclude.is_empty());
        assert!(config.include.is_empty());
    }

    #[test]
    fn test_should_include_entry_no_filters() {
        let config = Config::default();
        assert!(config.should_include_entry("key1"));
        assert!(config.should_include_entry("key2"));
    }

    #[test]
    fn test_should_include_entry_with_include() {
        let config = Config {
            include: vec!["key1".to_string(), "key2".to_string()],
            ..Default::default()
        };
        assert!(config.should_include_entry("key1"));
        assert!(config.should_include_entry("key2"));
        assert!(!config.should_include_entry("key3"));
    }

    #[test]
    fn test_should_include_entry_with_exclude() {
        let config = Config {
            exclude: vec!["key1".to_string()],
            ..Default::default()
        };
        assert!(!config.should_include_entry("key1"));
        assert!(config.should_include_entry("key2"));
    }

    #[test]
    fn test_from_cli_exclude_and_include_mutually_exclusive() {
        let result = Config::from_cli(
            Some("input.bib".to_string()),
            Some("output".to_string()),
            Some("template.md".to_string()),
            Some("key1,key2".to_string()),
            Some("key3,key4".to_string()),
            false,
            false,
            false,
        );

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot specify both --exclude and --include"));
    }

    #[test]
    fn test_validate_config_requires_all_fields() {
        let config = Config::default();
        assert!(config.validate().is_err());

        let mut config = Config {
            input: Some("input.bib".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.output = Some("output".to_string());
        assert!(config.validate().is_err());

        config.template = Some("template.md".to_string());
        // This will still fail because the files don't exist
        assert!(config.validate().is_err());
    }
}
