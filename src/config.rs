//! Configuration management module.
//!
//! This module provides validated runtime configuration derived from CLI arguments.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cli::{FileNameStrategy as CliFileNameStrategy, InfoArgs, TransformArgs};

/// Error types for configuration operations
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Error when configuration is invalid
    #[error("Invalid configuration: {0}")]
    Validation(String),
}

/// Output filename strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FileNameStrategy {
    /// UUID7 derived from SHAKE-128 output bytes
    #[default]
    Uuid7,
    /// Slugify the key by replacing non-alphanumeric characters with underscores
    Slugify,
}

impl From<CliFileNameStrategy> for FileNameStrategy {
    fn from(value: CliFileNameStrategy) -> Self {
        match value {
            CliFileNameStrategy::Uuid7 => Self::Uuid7,
            CliFileNameStrategy::Slugify => Self::Slugify,
        }
    }
}

/// Shared key filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    /// List of BibTeX entry keys to exclude from processing
    #[serde(default)]
    pub exclude: Vec<String>,
    /// List of BibTeX entry keys to include in processing
    #[serde(default)]
    pub include: Vec<String>,
}

impl FilterConfig {
    /// Build filter config from optional include/exclude CSV strings.
    pub fn from_options(exclude: Option<String>, include: Option<String>) -> Result<Self> {
        let exclude = parse_csv_list(exclude);
        let include = parse_csv_list(include);

        if !exclude.is_empty() && !include.is_empty() {
            return Err(ConfigError::Validation(
                "Cannot specify both --exclude and --include at the same time".to_string(),
            )
            .into());
        }

        Ok(Self { exclude, include })
    }

    /// Check if an entry key should be included.
    pub fn should_include_entry(&self, key: &str) -> bool {
        if !self.include.is_empty() {
            return self.include.iter().any(|k| k == key);
        }

        if !self.exclude.is_empty() {
            return !self.exclude.iter().any(|k| k == key);
        }

        true
    }
}

/// Runtime config for `transform`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Input BibTeX file path
    pub input: String,
    /// Output directory path
    pub output: String,
    /// Tera template path
    pub template: String,
    /// Include/exclude filtering
    pub filter: FilterConfig,
    /// Dry run mode
    pub dry_run: bool,
    /// Force overwrite mode
    pub overwrite: bool,
    /// Filename strategy
    pub file_name_strategy: FileNameStrategy,
    /// Render all selected entries into one output file
    pub single: bool,
    /// Verbose mode
    pub verbose: bool,
}

impl TransformConfig {
    /// Create and validate transform config from CLI args.
    pub fn from_args(args: TransformArgs) -> Result<Self> {
        let cfg = Self {
            input: args.input,
            output: args.output,
            template: args.template,
            filter: FilterConfig::from_options(args.exclude, args.include)?,
            dry_run: args.dry_run,
            overwrite: args.overwrite,
            file_name_strategy: args.file_name_strategy.into(),
            single: args.single,
            verbose: args.verbose,
        };

        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate transform config.
    pub fn validate(&self) -> Result<()> {
        if !self.input.ends_with(".bib") {
            return Err(ConfigError::Validation(format!(
                "Input must be a .bib file: {}",
                self.input
            ))
            .into());
        }

        if !Path::new(&self.input).exists() {
            return Err(ConfigError::Validation(format!(
                "Input path does not exist: {}",
                self.input
            ))
            .into());
        }

        if !Path::new(&self.template).exists() {
            return Err(ConfigError::Validation(format!(
                "Template path does not exist: {}",
                self.template
            ))
            .into());
        }

        Ok(())
    }
}

/// Runtime config for `info`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InfoConfig {
    /// Optional input BibTeX file path
    pub input: Option<String>,
    /// Include/exclude filtering
    pub filter: FilterConfig,
}

impl InfoConfig {
    /// Create and validate info config from CLI args.
    pub fn from_args(args: InfoArgs) -> Result<Self> {
        let cfg = Self {
            input: args.input,
            filter: FilterConfig::from_options(args.exclude, args.include)?,
        };

        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate info config.
    pub fn validate(&self) -> Result<()> {
        if let Some(input) = &self.input {
            if !input.ends_with(".bib") {
                return Err(ConfigError::Validation(format!(
                    "Input must be a .bib file: {}",
                    input
                ))
                .into());
            }

            if !Path::new(input).exists() {
                return Err(ConfigError::Validation(format!(
                    "Input path does not exist: {}",
                    input
                ))
                .into());
            }
        }

        Ok(())
    }
}

fn parse_csv_list(value: Option<String>) -> Vec<String> {
    value
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_include() {
        let filter = FilterConfig {
            include: vec!["a".to_string()],
            ..Default::default()
        };
        assert!(filter.should_include_entry("a"));
        assert!(!filter.should_include_entry("b"));
    }

    #[test]
    fn test_filter_exclude() {
        let filter = FilterConfig {
            exclude: vec!["a".to_string()],
            ..Default::default()
        };
        assert!(!filter.should_include_entry("a"));
        assert!(filter.should_include_entry("b"));
    }

    #[test]
    fn test_filter_mutually_exclusive() {
        let result = FilterConfig::from_options(Some("a".to_string()), Some("b".to_string()));
        assert!(result.is_err());
    }
}
