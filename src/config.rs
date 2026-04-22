//! Configuration management module.
//!
//! This module provides configuration management for the BibTeX converter,
//! including command-line overrides, default settings, and configuration files.

use std::env;
use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Import cli module for OutputFormat conversion
use crate::cli;

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

/// Output format configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum OutputFormat {
    /// Markdown output (.md)
    #[default]
    Markdown,
    /// HTML output (.html)
    Html,
    /// JSON output (.json)
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Markdown => write!(f, "markdown"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

impl From<String> for OutputFormat {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => OutputFormat::Markdown,
            "html" | "htm" => OutputFormat::Html,
            "json" => OutputFormat::Json,
            _ => OutputFormat::Markdown, // Default to markdown
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Input BibTeX file or directory path
    #[serde(default)]
    pub input: Option<String>,

    /// Output file or directory path
    #[serde(default)]
    pub output: Option<String>,

    /// Custom template file path
    #[serde(default)]
    pub template: Option<String>,

    /// Output format (default: markdown)
    #[serde(default)]
    pub format: OutputFormat,

    /// Process files recursively
    #[serde(default = "default_true")]
    pub recursive: bool,

    /// Enable verbose output
    #[serde(default = "default_true")]
    pub verbose: bool,

    /// Template directory for custom templates
    #[serde(default = "default_templates_dir")]
    pub templates_dir: String,

    /// Encoding for input/output files
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

/// Default values helper functions
fn default_true() -> bool {
    false
}

fn default_templates_dir() -> String {
    "templates".to_string()
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            template: None,
            format: OutputFormat::Markdown,
            recursive: true,
            verbose: true,
            templates_dir: "templates".to_string(),
            encoding: "utf-8".to_string(),
        }
    }
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config from command-line arguments
    pub fn from_cli(
        input: Option<String>,
        output: Option<String>,
        template: Option<String>,
        format: cli::OutputFormat,
        recursive: bool,
        verbose: bool,
    ) -> Self {
        // Convert CLI output format to config output format
        let format = match format {
            cli::OutputFormat::Markdown => OutputFormat::Markdown,
            cli::OutputFormat::Html => OutputFormat::Html,
            cli::OutputFormat::Json => OutputFormat::Json,
        };
        Self {
            input,
            output,
            template,
            format,
            recursive,
            verbose,
            ..Self::default()
        }
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
                ConfigError::Validation("Input file or directory is required".to_string()).into(),
            );
        }

        // Check output is provided
        if self.output.is_none() {
            return Err(ConfigError::Validation(
                "Output file or directory is required".to_string(),
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

        Ok(())
    }

    /// Get the output file extension based on format
    pub fn output_extension(&self) -> &'static str {
        match self.format {
            OutputFormat::Markdown => "md",
            OutputFormat::Html => "html",
            OutputFormat::Json => "json",
        }
    }

    /// Get the output filename for a given input filename
    pub fn output_filename(&self, input_filename: &str) -> String {
        let stem = input_filename
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(input_filename);

        format!("{}.{}", stem, self.output_extension())
    }

    /// Build the output path
    pub fn build_output_path(&self) -> Option<std::path::PathBuf> {
        let output = self.output.as_ref()?;

        if let Some(parent) = std::path::Path::new(output).parent()
            && !parent.exists()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!("Warning: Could not create output directory: {}", e);
        }

        Some(std::path::PathBuf::from(output))
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Config {{\n  input: {:?},\n  output: {:?},\n  template: {:?},\n  format: {:?},\n  recursive: {},\n  verbose: {},\n  templates_dir: {},\n  encoding: {}\n}}",
            self.input,
            self.output,
            self.template,
            self.format,
            self.recursive,
            self.verbose,
            self.templates_dir,
            self.encoding
        )
    }
}

/// Environment configuration provider
pub struct EnvConfig;

impl EnvConfig {
    /// Get environment variable as string
    pub fn get(var: &str) -> Option<String> {
        env::var(var).ok()
    }

    /// Get environment variable as typed value
    pub fn get_typed<T: std::str::FromStr>(var: &str) -> Option<T> {
        env::var(var).ok()?.parse().ok()
    }

    /// Get environment variable with default value
    pub fn get_with_default<T: std::str::FromStr>(var: &str, default: T) -> T {
        env::var(var)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    /// Check if environment variable is set
    pub fn is_set(var: &str) -> bool {
        env::var(var).is_ok()
    }
}

/// Configuration file paths provider
pub struct ConfigPaths;

impl ConfigPaths {
    /// Get the user's home directory config path
    pub fn home_config() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("bibtera"))
    }

    /// Get the config file path
    pub fn config_file() -> Option<std::path::PathBuf> {
        Self::home_config().map(|p| p.join("config.json"))
    }

    /// Get the templates directory path
    pub fn templates_dir() -> Option<std::path::PathBuf> {
        Self::home_config().map(|p| p.join("templates"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.format, OutputFormat::Markdown);
        assert!(config.recursive);
        assert!(config.verbose);
    }

    #[test]
    fn test_cli_config() {
        let config = Config::from_cli(
            Some("input.bib".to_string()),
            Some("output.md".to_string()),
            None,
            crate::cli::OutputFormat::Html,
            false,
            false,
        );

        assert_eq!(config.input, Some("input.bib".to_string()));
        assert_eq!(config.output, Some("output.md".to_string()));
        assert_eq!(config.format, OutputFormat::Html);
        assert!(!config.recursive);
        assert!(!config.verbose);
    }

    #[test]
    fn test_output_extension() {
        let mut config = Config::default();
        assert_eq!(config.output_extension(), "md");

        config.format = OutputFormat::Html;
        assert_eq!(config.output_extension(), "html");

        config.format = OutputFormat::Json;
        assert_eq!(config.output_extension(), "json");
    }

    #[test]
    fn test_output_filename() {
        let config = Config::default();
        assert_eq!(config.output_filename("paper.bib"), "paper.md");
        assert_eq!(config.output_filename("citation.bib"), "citation.md");
    }

    #[test]
    fn test_format_from_string() {
        assert_eq!(
            OutputFormat::from("markdown".to_string()),
            OutputFormat::Markdown
        );
        assert_eq!(OutputFormat::from("md".to_string()), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from("html".to_string()), OutputFormat::Html);
        assert_eq!(OutputFormat::from("json".to_string()), OutputFormat::Json);
        assert_eq!(
            OutputFormat::from("invalid".to_string()),
            OutputFormat::Markdown
        );
    }

    #[test]
    fn test_validate_config() {
        let mut config = Config::default();

        // Should fail without input
        assert!(config.validate().is_err());

        // Should fail without output
        config.input = Some("test.bib".to_string());
        assert!(config.validate().is_err());

        // Should pass with both when input path exists
        let temp_input = std::env::temp_dir().join("bibtera_test_validate_input.bib");
        std::fs::write(
            &temp_input,
            "@article{k, title={t}, author={a}, year={2024}}",
        )
        .unwrap();
        config.input = Some(temp_input.to_string_lossy().to_string());
        config.output = Some("output.md".to_string());
        assert!(config.validate().is_ok());
        let _ = std::fs::remove_file(temp_input);
    }

    #[test]
    fn test_env_config() {
        assert!(EnvConfig::is_set("PATH"));
        assert!(EnvConfig::get("PATH").is_some());
    }
}
