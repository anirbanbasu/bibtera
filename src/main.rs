//! BibTeX to Markdown Converter
//!
//! This application parses BibTeX entries and generates output in any text-based format
//! using customizable Tera templates. The generated output can be used by static site
//! generators like Zola.
//!
//! # Example
//!
//! ```bash
//! # Convert a BibTeX file using a template
//! bibtera -i citations.bib -o output/ -t template.md
//!
//! # Use include to process only specific entries
//! bibtera -i citations.bib -o output/ -t template.md --include key1,key2
//!
//! # Use exclude to skip specific entries
//! bibtera -i citations.bib -o output/ -t template.md --exclude key1
//!
//! # Perform a dry run to see what would be generated
//! bibtera -i citations.bib -o output/ -t template.md --dry-run
//! ```

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use bibtera::cli::Cli;
use bibtera::config::Config;
use bibtera::parser::BibTeXParser;
use bibtera::template::TemplateEngine;
use bibtera::utils;

fn main() {
    match run() {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

/// Run the main application logic
fn run() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Create configuration from CLI arguments
    let config = Config::from_cli(
        cli.input,
        cli.output,
        cli.template,
        cli.exclude,
        cli.include,
        cli.dry_run,
        cli.overwrite,
        cli.verbose,
    )?;

    // Initialize logger (if verbose)
    if config.verbose {
        eprintln!("BibTeX Converter v0.1.0");
        eprintln!("Configuration: {:?}", config);
    }

    // Validate configuration
    config.validate().context("Invalid configuration")?;

    // Initialize the template engine
    let mut template_engine =
        TemplateEngine::new().context("Failed to initialize template engine")?;

    // Load custom template if specified
    if let Some(template_path) = &config.template {
        template_engine
            .add_template(template_path)
            .context("Failed to load custom template")?;
        if config.verbose {
            eprintln!("Loaded template: {}", template_path);
        }
    }

    // Parse BibTeX entries
    let entries = parse_bibtex(&config)?;

    // Filter entries based on include/exclude
    let filtered_entries: Vec<_> = entries
        .iter()
        .filter(|entry| config.should_include_entry(&entry.key))
        .collect();

    if config.verbose {
        eprintln!(
            "Processing {} entries (filtered from {})",
            filtered_entries.len(),
            entries.len()
        );
    }

    // Render entries to individual files
    render_entries(&config, &template_engine, &filtered_entries)?;

    if config.verbose {
        eprintln!("Successfully processed {} entries", filtered_entries.len());
    }

    Ok(())
}

/// Parse BibTeX entries from input file
fn parse_bibtex(config: &Config) -> Result<Vec<bibtera::parser::BibTeXEntry>> {
    let input = config.input.as_ref().context("Input is required")?;

    if !input.ends_with(".bib") {
        anyhow::bail!("Input must be a .bib file: {}", input);
    }

    BibTeXParser::parse_file(input)
}

/// Render entries to individual output files
fn render_entries(
    config: &Config,
    template_engine: &TemplateEngine,
    entries: &[&bibtera::parser::BibTeXEntry],
) -> Result<()> {
    let output_dir = config
        .output
        .as_ref()
        .context("Output directory is required")?;
    let template_path = config.template.as_ref().context("Template is required")?;

    // Get file extension from template file
    let template_extension =
        utils::extension(template_path).context("Template file must have an extension")?;

    // Create output directory if it doesn't exist
    if !config.dry_run {
        std::fs::create_dir_all(output_dir).context("Failed to create output directory")?;
    }

    let template_name = std::path::Path::new(template_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .context("Invalid template path")?;

    // Process each entry
    for entry in entries {
        // Generate unique filename using SHA-256 hash of the key
        let filename = utils::generate_unique_filename(&entry.key, &template_extension);
        let output_path = PathBuf::from(output_dir).join(&filename);

        if config.verbose {
            eprintln!("Processing: {} -> {}", entry.key, filename);
        }

        if config.dry_run {
            println!("{} -> {}", entry.key, filename);
            continue;
        }

        // Check if file exists and handle overwrite flag
        if output_path.exists() && !config.overwrite {
            eprintln!(
                "Warning: File already exists, skipping: {}",
                output_path.display()
            );
            continue;
        }

        // Render the entry
        let rendered = template_engine
            .render_entry(template_name, entry)
            .context(format!("Failed to render entry: {}", entry.key))?;

        // Write to file
        utils::safe_write(&output_path, &rendered).context(format!(
            "Failed to write output file: {}",
            output_path.display()
        ))?;

        if config.verbose {
            eprintln!("Written: {}", output_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bibtex_single_file() {
        let temp_file = std::env::temp_dir().join("bibtera_test_main_parse.bib");
        std::fs::write(
            &temp_file,
            "@article{t, author={A}, title={T}, year={2024}}",
        )
        .unwrap();

        let config = Config {
            input: Some(temp_file.to_string_lossy().to_string()),
            output: Some("output".to_string()),
            template: Some("template.md".to_string()),
            ..Default::default()
        };

        let result = parse_bibtex(&config);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "t");

        let _ = std::fs::remove_file(temp_file);
    }
}
