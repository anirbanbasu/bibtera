//! BibTeX to Markdown Converter
//!
//! This application parses BibTeX entries and generates output in various formats
//! (Markdown, HTML, JSON) using Tera templates. The generated output can be used
//! by static site generators like Zola.
//!
//! # Example
//!
//! ```bash
//! # Convert a single BibTeX file
//! bibtera -i citations.bib -o output.md
//!
//! # Convert all BibTeX files in a directory
//! bibtera -i ./bibs/ -o ./output/ --recursive
//!
//! # Use a custom template
//! bibtera -i citations.bib -o output.md -t my_template.md
//! ```

use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use bibtera::cli::{Cli, Commands, OutputFormat as CliOutputFormat};
use bibtera::config::{Config, OutputFormat as ConfigOutputFormat};
use bibtera::parser::{BibTeXEntry, BibTeXParser};
use bibtera::template::{
    HtmlHandler, JsonHandler, MarkdownHandler, TemplateEngine, render_entries_with_handler,
};

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

    // Handle subcommands that should not require full conversion config.
    match &cli.command {
        Some(Commands::InitTemplates { output_dir }) => {
            init_templates(output_dir)?;
            return Ok(());
        }
        Some(Commands::Validate { input }) => {
            validate_bibtex(input, cli.verbose)?;
            return Ok(());
        }
        None => {}
    }

    // Create configuration from CLI arguments
    let config = Config::from_cli(
        cli.input.clone(),
        cli.output.clone(),
        cli.template.clone(),
        cli.format,
        cli.recursive,
        cli.verbose,
    );

    // Initialize logger (if verbose)
    if config.verbose {
        eprintln!("BibTeX to Markdown Converter v0.1.0");
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
            eprintln!("Loaded custom template: {}", template_path);
        }
    }

    // Parse BibTeX entries
    let entries = parse_bibtex(&config)?;

    // Render entries - convert config output format to cli output format for matching
    render_entries(&config, &template_engine, &entries)?;

    if config.verbose {
        eprintln!("Successfully processed {} entries", entries.len());
    }

    Ok(())
}

/// Parse BibTeX entries from input
fn parse_bibtex(config: &Config) -> Result<Vec<BibTeXEntry>> {
    let input = config.input.as_ref().context("Input is required")?;

    if input.ends_with(".bib") {
        // Single file
        let entries = BibTeXParser::parse_file(input)?;
        Ok(entries)
    } else if std::path::Path::new(input).is_dir() {
        // Directory
        let entries = BibTeXParser::parse_directory(input, config.recursive)?;
        Ok(entries)
    } else {
        anyhow::bail!("Invalid input path: {}", input);
    }
}

/// Render entries to output
fn render_entries(
    config: &Config,
    template_engine: &TemplateEngine,
    entries: &[BibTeXEntry],
) -> Result<()> {
    let output = config.output.as_ref().context("Output is required")?;

    // Use user-supplied template when provided.
    if let Some(template_path) = &config.template {
        let template_name = std::path::Path::new(template_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .context("Invalid custom template path")?;

        let rendered_entries = entries
            .iter()
            .map(|entry| template_engine.render_entry(template_name, entry))
            .collect::<Result<Vec<_>>>()
            .context("Failed to render entries with custom template")?;
        let rendered = rendered_entries.join("\n\n");

        std::fs::write(output, rendered).context("Failed to write output file")?;
        return Ok(());
    }

    // Convert config output format to cli output format for matching
    let cli_format = match config.format {
        ConfigOutputFormat::Markdown => CliOutputFormat::Markdown,
        ConfigOutputFormat::Html => CliOutputFormat::Html,
        ConfigOutputFormat::Json => CliOutputFormat::Json,
    };

    match cli_format {
        CliOutputFormat::Markdown => {
            let handler = MarkdownHandler::new();
            render_entries_with_handler(&handler, entries, output)?;
        }
        CliOutputFormat::Html => {
            let handler = HtmlHandler::new();
            render_entries_with_handler(&handler, entries, output)?;
        }
        CliOutputFormat::Json => {
            let handler = JsonHandler::new();
            render_entries_with_handler(&handler, entries, output)?;
        }
    }

    Ok(())
}

/// Initialize default templates
fn init_templates(output_dir: &str) -> Result<()> {
    use std::fs;

    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create templates directory")?;

    // Create template files
    let templates = vec![
        (
            "bibtex_entry.md",
            include_str!("../templates/bibtex_entry.md"),
        ),
        (
            "bibtex_entry.html",
            include_str!("../templates/bibtex_entry.html"),
        ),
        (
            "bibtex_entry.json",
            include_str!("../templates/bibtex_entry.json"),
        ),
    ];

    for (name, content) in templates {
        let path = format!("{}/{}", output_dir, name);
        fs::write(&path, content).context(format!("Failed to write {}", name))?;
        eprintln!("Created template: {}", path);
    }

    eprintln!("Templates initialized successfully!");

    Ok(())
}

/// Validate a BibTeX file
fn validate_bibtex(input: &str, verbose: bool) -> Result<()> {
    if verbose {
        eprintln!("Validating BibTeX file: {}", input);
    }

    match BibTeXParser::parse_file(input) {
        Ok(entries) => {
            eprintln!("Valid! Found {} entries", entries.len());
            for entry in &entries {
                eprintln!("  - {} ({})", entry.key, entry.entry_type);
            }
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Validation failed: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bibtex_single_file() {
        // Use a real temp file so config validation constraints are respected.
        let temp_file = std::env::temp_dir().join("bibtera_test_main_parse.bib");
        std::fs::write(
            &temp_file,
            "@article{t, author={A}, title={T}, year={2024}}",
        )
        .unwrap();
        let config = Config::from_cli(
            Some(temp_file.to_string_lossy().to_string()),
            None,
            None,
            CliOutputFormat::Markdown,
            false,
            false,
        );
        assert!(config.input.is_some());
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entries() {
        let config = Config::from_cli(
            None,
            Some("output.md".to_string()),
            None,
            CliOutputFormat::Markdown,
            false,
            false,
        );

        let entries = vec![
            BibTeXEntry::new(
                "test1".to_string(),
                "article".to_string(),
                vec!["Author One".to_string()],
                "Test Title".to_string(),
            ),
            BibTeXEntry::new(
                "test2".to_string(),
                "book".to_string(),
                vec!["Author Two".to_string()],
                "Book Title".to_string(),
            ),
        ];

        let template_engine = TemplateEngine::new().unwrap();
        assert!(render_entries(&config, &template_engine, &entries).is_ok());
    }
}
