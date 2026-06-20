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
    /// List of BibTeX entry types to exclude from processing
    #[serde(default)]
    pub exclude_types: Vec<String>,
    /// List of BibTeX entry types to include in processing
    #[serde(default)]
    pub include_types: Vec<String>,
}

impl FilterConfig {
    /// Build filter config from optional include/exclude key and type CSV strings.
    pub fn from_options(
        exclude: Option<String>,
        include: Option<String>,
        exclude_type: Option<String>,
        include_type: Option<String>,
    ) -> Result<Self> {
        let exclude = parse_csv_list(exclude);
        let include = parse_csv_list(include);
        let exclude_types = parse_csv_type_list(exclude_type);
        let include_types = parse_csv_type_list(include_type);

        if !exclude.is_empty() && !include.is_empty() {
            return Err(ConfigError::Validation(
                "Cannot specify both --exclude and --include at the same time".to_string(),
            )
            .into());
        }

        if !exclude_types.is_empty() && !include_types.is_empty() {
            return Err(ConfigError::Validation(
                "Cannot specify both --exclude-type and --include-type at the same time"
                    .to_string(),
            )
            .into());
        }

        Ok(Self {
            exclude,
            include,
            exclude_types,
            include_types,
        })
    }

    /// Check if an entry should be included based on key and entry type filters.
    pub fn should_include_entry(&self, key: &str, entry_type: &str) -> bool {
        let normalised_entry_type = normalise_entry_type(entry_type);

        if !self.include.is_empty() && !self.include.iter().any(|k| k == key) {
            return false;
        }

        if !self.include_types.is_empty()
            && !self
                .include_types
                .iter()
                .any(|entry_type| entry_type == &normalised_entry_type)
        {
            return false;
        }

        if self.exclude.iter().any(|k| k == key) {
            return false;
        }

        if self
            .exclude_types
            .iter()
            .any(|entry_type| entry_type == &normalised_entry_type)
        {
            return false;
        }

        true
    }

    /// Return true when any explicit include/exclude selector is configured.
    pub fn has_explicit_selection(&self) -> bool {
        !self.include.is_empty()
            || !self.exclude.is_empty()
            || !self.include_types.is_empty()
            || !self.exclude_types.is_empty()
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
    /// Optional path to a JSON file with LaTeX substitution overrides
    pub latex_substitution_map: Option<String>,
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
            filter: FilterConfig::from_options(
                args.exclude,
                args.include,
                args.exclude_type,
                args.include_type,
            )?,
            dry_run: args.dry_run,
            overwrite: args.overwrite,
            file_name_strategy: args.file_name_strategy.into(),
            single: args.single,
            latex_substitution_map: args.latex_substitution_map,
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

        if let Some(map_path) = &self.latex_substitution_map
            && !Path::new(map_path).exists()
        {
            return Err(ConfigError::Validation(format!(
                "LaTeX substitution map path does not exist: {}",
                map_path
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
            filter: FilterConfig::from_options(
                args.exclude,
                args.include,
                args.exclude_type,
                args.include_type,
            )?,
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

fn parse_csv_type_list(value: Option<String>) -> Vec<String> {
    parse_csv_list(value)
        .into_iter()
        .map(|entry_type| normalise_entry_type(&entry_type))
        .collect()
}

fn normalise_entry_type(entry_type: &str) -> String {
    entry_type.trim().to_ascii_lowercase()
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
        assert!(filter.should_include_entry("a", "article"));
        assert!(!filter.should_include_entry("b", "article"));
    }

    #[test]
    fn test_filter_exclude() {
        let filter = FilterConfig {
            exclude: vec!["a".to_string()],
            ..Default::default()
        };
        assert!(!filter.should_include_entry("a", "article"));
        assert!(filter.should_include_entry("b", "article"));
    }

    #[test]
    fn test_filter_mutually_exclusive() {
        let result =
            FilterConfig::from_options(Some("a".to_string()), Some("b".to_string()), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_type_include() {
        let filter = FilterConfig {
            include_types: vec!["article".to_string()],
            ..Default::default()
        };

        assert!(filter.should_include_entry("a", "article"));
        assert!(!filter.should_include_entry("a", "book"));
    }

    #[test]
    fn test_filter_type_exclude() {
        let filter = FilterConfig {
            exclude_types: vec!["article".to_string()],
            ..Default::default()
        };

        assert!(!filter.should_include_entry("a", "article"));
        assert!(filter.should_include_entry("a", "book"));
    }

    #[test]
    fn test_filter_type_mutually_exclusive() {
        let result = FilterConfig::from_options(
            None,
            None,
            Some("article".to_string()),
            Some("book".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_combines_key_and_type_constraints() {
        let filter = FilterConfig {
            include: vec!["a".to_string()],
            include_types: vec!["article".to_string()],
            ..Default::default()
        };

        assert!(filter.should_include_entry("a", "article"));
        assert!(!filter.should_include_entry("a", "book"));
        assert!(!filter.should_include_entry("b", "article"));
    }

    #[test]
    fn test_filter_type_matching_is_case_insensitive() {
        let filter = FilterConfig {
            include_types: vec!["article".to_string()],
            ..Default::default()
        };

        assert!(filter.should_include_entry("a", "Article"));
        assert!(filter.should_include_entry("a", "ARTICLE"));
    }

    #[test]
    fn test_filter_has_explicit_selection_for_type_filters() {
        let filter = FilterConfig {
            include_types: vec!["article".to_string()],
            ..Default::default()
        };

        assert!(filter.has_explicit_selection());
    }
}
