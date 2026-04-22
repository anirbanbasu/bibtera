//! Command-line interface for BibTeX to Markdown converter.
//!
//! This module defines the command-line arguments using the Clap library.
//! It supports input files, output directories, template files, and filtering options.

use clap::Parser;

/// BibTeX to Markdown converter using Tera templates.
#[derive(Parser, Debug)]
#[command(name = "bibtera")]
#[command(author = "Anirban Basu")]
#[command(version = "0.1.0")]
#[command(about = "Parse BibTeX entries and generate output using Tera templates")]
#[command(
    long_about = "Parse BibTeX entries from input files and generate output in any text-based format using customisable Tera templates. The generated output can be used by static site generators like Zola."
)]
pub struct Cli {
    /// Path to the input BibTeX file
    #[arg(short, long)]
    pub input: Option<String>,

    /// Path to the output directory where generated files will be saved
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to the Tera template file used for formatting each file in the output directory
    #[arg(short, long)]
    pub template: Option<String>,

    /// Comma-separated list of BibTeX entry keys to exclude from processing
    #[arg(long)]
    pub exclude: Option<String>,

    /// Comma-separated list of BibTeX entry keys to include in processing. If specified, only these entries will be processed.
    #[arg(long)]
    pub include: Option<String>,

    /// Perform a dry run without generating any files, but print the intended output file names and their corresponding BibTeX entry keys
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Force overwrite of existing files in the output directory without prompting
    #[arg(short = 'f', long)]
    pub overwrite: bool,

    /// Enable verbose logging for debugging purposes
    #[arg(short, long)]
    pub verbose: bool,
}
