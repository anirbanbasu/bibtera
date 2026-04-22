//! Command-line interface for BibTeX to Markdown converter.
//!
//! This module defines the command-line arguments using the Clap library.
//! It supports input files, output directories, template files, and output formats.

use clap::{Parser, Subcommand, ValueEnum};

/// BibTeX to Markdown converter using Tera templates.
#[derive(Parser, Debug)]
#[command(name = "bibtera")]
#[command(author = "Anirban Basu")]
#[command(version = "0.1.0")]
#[command(about = "Parse BibTeX entries and generate Markdown output using Tera templates")]
#[command(
    long_about = "Parse BibTeX entries from input files and generate output in Markdown (or other formats) using customizable Tera templates. The generated output can be used by static site generators like Zola."
)]
pub struct Cli {
    /// Input BibTeX file or directory
    #[arg(short, long)]
    pub input: Option<String>,

    /// Output directory or file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to custom Tera template file
    #[arg(short, long)]
    pub template: Option<String>,

    /// Output format (default: markdown)
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Markdown)]
    pub format: OutputFormat,

    /// Process all .bib files in input directory (only works with directory input)
    #[arg(short, long, default_value_t = false)]
    pub recursive: bool,

    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Subcommands for extended functionality
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Supported output formats
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum OutputFormat {
    /// Generate Markdown output
    Markdown,
    /// Generate HTML output
    Html,
    /// Generate JSON output
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

/// Subcommands for extended functionality
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize default templates
    InitTemplates {
        /// Output directory for templates
        #[arg(short, long, default_value = "templates")]
        output_dir: String,
    },
    /// Validate BibTeX file
    Validate {
        /// BibTeX file to validate
        #[arg(short, long)]
        input: String,
    },
}
