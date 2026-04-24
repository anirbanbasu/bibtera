//! Command-line interface for BibTeX converter.
//!
//! This module defines command-line arguments using Clap.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// BibTeX converter using Tera templates.
#[derive(Parser, Debug)]
#[command(name = "bibtera")]
#[command(author = "Anirban Basu")]
#[command(version = "0.1.0")]
#[command(about = "Parse BibTeX entries and generate output using Tera templates")]
#[command(
    long_about = "Parse BibTeX entries from input files and generate output in any text-based format using customisable Tera templates. The generated output can be used by static site generators like Zola."
)]
pub struct Cli {
    /// Available subcommands
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Transform BibTeX entries to files using a Tera template
    Transform(TransformArgs),
    /// Display BibTeX entry information available to Tera templates
    Info(InfoArgs),
}

/// File naming strategy for generated output files
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FileNameStrategy {
    /// UUID7 derived from SHAKE-128 hash bytes
    Uuid7,
    /// Replace non-alphanumeric key characters with underscores
    Slugify,
}

/// Arguments for transform subcommand
#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Path to the input BibTeX file
    #[arg(short, long)]
    pub input: String,

    /// Path to output directory where generated files will be written
    #[arg(short, long)]
    pub output: String,

    /// Path to the Tera template file
    #[arg(short, long)]
    pub template: String,

    /// Comma-separated BibTeX keys to exclude
    #[arg(long)]
    pub exclude: Option<String>,

    /// Comma-separated BibTeX keys to include
    #[arg(long)]
    pub include: Option<String>,

    /// Perform a dry run without generating any files, but print the intended output file names and their corresponding BibTeX entry keys
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Force overwrite of existing files without prompting
    #[arg(short = 'f', long)]
    pub overwrite: bool,

    /// Output filename generation strategy
    #[arg(long, value_enum, default_value_t = FileNameStrategy::Uuid7)]
    pub file_name_strategy: FileNameStrategy,

    /// Render all selected entries in a single output file via the template `entries` variable
    #[arg(long)]
    pub single: bool,

    /// Enable verbose transformation logs
    #[arg(short, long)]
    pub verbose: bool,
}

/// Arguments for info subcommand
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Path to the input BibTeX file
    #[arg(short, long)]
    pub input: Option<String>,

    /// Comma-separated BibTeX keys to exclude
    #[arg(long)]
    pub exclude: Option<String>,

    /// Comma-separated BibTeX keys to include
    #[arg(long)]
    pub include: Option<String>,
}
