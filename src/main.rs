//! BibTeX converter CLI entry point.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;

use bibtera::cli::{Cli, Commands};
use bibtera::config::{InfoConfig, TransformConfig};
use bibtera::parser::BibTeXParser;
use bibtera::template::TemplateEngine;
use bibtera::utils;

fn main() {
    let cli = Cli::parse();
    let verbose = matches!(&cli.command, Commands::Transform(args) if args.verbose);

    match run(cli) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);

            if verbose {
                let causes = e.chain().skip(1).collect::<Vec<_>>();
                if !causes.is_empty() {
                    eprintln!("Caused by:");
                    for (index, cause) in causes.iter().enumerate() {
                        eprintln!("  {}: {}", index + 1, cause);
                    }
                }
            }

            process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Transform(args) => {
            let config = TransformConfig::from_args(args)?;
            run_transform(config)
        }
        Commands::Info(args) => {
            let config = InfoConfig::from_args(args)?;
            run_info(config)
        }
    }
}

fn run_transform(config: TransformConfig) -> Result<()> {
    if config.verbose {
        eprintln!("Configuration: {:?}", config);
    }

    let mut template_engine =
        TemplateEngine::new().context("Failed to initialize template engine")?;
    template_engine
        .add_template(&config.template)
        .with_context(|| format!("Failed to load template: {}", config.template))?;

    let entries = BibTeXParser::parse_file(&config.input).context("Failed to parse BibTeX file")?;
    let filtered_entries = entries
        .iter()
        .filter(|entry| config.filter.should_include_entry(&entry.key))
        .collect::<Vec<_>>();

    if config.verbose {
        eprintln!(
            "Processing {} entries (filtered from {})",
            filtered_entries.len(),
            entries.len()
        );
    }

    render_entries(&config, &template_engine, &filtered_entries)?;

    if config.verbose {
        eprintln!("Successfully processed {} entries", filtered_entries.len());
    }

    Ok(())
}

fn render_entries(
    config: &TransformConfig,
    template_engine: &TemplateEngine,
    entries: &[&bibtera::parser::BibTeXEntry],
) -> Result<()> {
    let template_extension =
        utils::extension(&config.template).context("Template file must have an extension")?;

    if !config.dry_run {
        std::fs::create_dir_all(&config.output).context("Failed to create output directory")?;
    }

    let template_name = std::path::Path::new(&config.template)
        .file_stem()
        .and_then(|name| name.to_str())
        .context("Invalid template path")?;

    // Sequential processing keeps output order predictable based on input order.
    for entry in entries {
        let filename = utils::generate_output_filename(
            &entry.key,
            config.file_name_strategy,
            &template_extension,
        );
        let output_path = PathBuf::from(&config.output).join(&filename);

        if config.verbose {
            eprintln!("Processing: {} -> {}", entry.key, filename);
        }

        if config.dry_run {
            println!("{} -> {}", entry.key, filename);
            continue;
        }

        if output_path.exists() && !config.overwrite && !confirm_overwrite(&output_path)? {
            eprintln!("Warning: Skipped existing file: {}", output_path.display());
            continue;
        }

        let rendered = template_engine
            .render_entry(template_name, entry)
            .with_context(|| format!("Failed to render entry: {}", entry.key))?;

        utils::safe_write(&output_path, rendered.as_bytes())
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    }

    Ok(())
}

fn confirm_overwrite(path: &std::path::Path) -> Result<bool> {
    print!("File {} exists. Overwrite? [y/N]: ", path.display());
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read user confirmation")?;

    let answer = input.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn run_info(config: InfoConfig) -> Result<()> {
    if let Some(input) = &config.input {
        let entries = BibTeXParser::parse_file(input).context("Failed to parse BibTeX file")?;
        let selected = entries
            .iter()
            .filter(|entry| config.filter.should_include_entry(&entry.key))
            .collect::<Vec<_>>();

        if !selected.is_empty() {
            let mut by_key = BTreeMap::new();
            for entry in selected {
                by_key.insert(&entry.key, entry);
            }

            println!("{}", serde_json::to_string_pretty(&by_key)?);
            return Ok(());
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&default_entry_type_field_map())?
    );
    Ok(())
}

fn default_entry_type_field_map() -> BTreeMap<String, BTreeMap<String, String>> {
    let types = vec![
        (
            "article",
            vec![
                "author", "title", "journal", "year", "volume", "number", "pages",
            ],
        ),
        (
            "book",
            vec![
                "author",
                "editor",
                "title",
                "publisher",
                "year",
                "address",
                "edition",
            ],
        ),
        (
            "inproceedings",
            vec!["author", "title", "booktitle", "year", "pages", "publisher"],
        ),
        (
            "incollection",
            vec!["author", "title", "booktitle", "publisher", "year", "pages"],
        ),
        (
            "phdthesis",
            vec!["author", "title", "school", "year", "address"],
        ),
        (
            "mastersthesis",
            vec!["author", "title", "school", "year", "address"],
        ),
        (
            "techreport",
            vec!["author", "title", "institution", "year", "number"],
        ),
        (
            "misc",
            vec!["author", "title", "howpublished", "year", "note"],
        ),
    ];

    let mut map = BTreeMap::new();
    for (entry_type, fields) in types {
        let inner = template_available_fields_for_type(&fields);
        map.insert(entry_type.to_string(), inner);
    }

    map
}

fn template_available_fields_for_type(fields: &[&str]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    // Top-level keys available directly in Tera context.
    map.insert("key".to_string(), "string".to_string());
    map.insert("entry_type".to_string(), "string".to_string());
    map.insert("title".to_string(), "string".to_string());
    map.insert("authors".to_string(), "array<string>".to_string());
    map.insert(
        "author_parts".to_string(),
        "array<{first:string,last:string,full:string}>".to_string(),
    );
    map.insert("year".to_string(), "string|null".to_string());
    map.insert("raw_bibtex".to_string(), "string".to_string());
    map.insert("fields".to_string(), "map<string,string>".to_string());

    // Known type-specific BibTeX fields exposed under `fields`.
    for field in BTreeSet::from_iter(fields.iter().copied()) {
        map.insert(format!("fields.{}", field), "string".to_string());
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_entry_type_field_map() {
        let map = default_entry_type_field_map();
        assert!(map.contains_key("article"));
        assert!(
            map.get("article")
                .expect("article map")
                .contains_key("fields.author")
        );
        assert!(
            map.get("article")
                .expect("article map")
                .contains_key("author_parts")
        );
    }
}
