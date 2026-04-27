//! BibTeX converter CLI entry point.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use bibtera::cli::{Cli, Commands};
use bibtera::config::{InfoConfig, TransformConfig};
use bibtera::latex;
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
    let started_at = Instant::now();

    if config.verbose {
        eprintln!("Configuration: {:?}", config);
    }

    let custom_substitutions = config
        .latex_substitution_map
        .as_deref()
        .map(|map_path| latex::load_substitution_map_file(std::path::Path::new(map_path)))
        .transpose()
        .context("Failed to load custom LaTeX substitution map")?;

    let mut template_engine = TemplateEngine::new_with_substitutions(custom_substitutions)
        .context("Failed to initialise template engine")?;
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

    let stats = render_entries(&config, &template_engine, &filtered_entries)?;

    if config.verbose {
        eprintln!("Successfully processed {} entries", filtered_entries.len());
    }

    eprintln!(
        "Summary: processed {} entries, generated {} files, total time {:?}",
        stats.entries_processed,
        stats.files_generated,
        started_at.elapsed()
    );

    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct TransformStats {
    entries_processed: usize,
    files_generated: usize,
}

fn render_entries(
    config: &TransformConfig,
    template_engine: &TemplateEngine,
    entries: &[&bibtera::parser::BibTeXEntry],
) -> Result<TransformStats> {
    let template_extension =
        utils::extension(&config.template).context("Template file must have an extension")?;

    if !config.dry_run {
        std::fs::create_dir_all(&config.output).context("Failed to create output directory")?;
    }

    let template_name = std::path::Path::new(&config.template)
        .file_stem()
        .and_then(|name| name.to_str())
        .context("Invalid template path")?;

    let mut stats = TransformStats::default();

    if config.single {
        stats.entries_processed = entries.len();

        let progress = if config.verbose {
            None
        } else {
            let pb = ProgressBar::new(1);
            pb.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] {wide_bar} {pos}/{len} tasks | files generated: {msg}",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=>-"),
            );
            pb.set_message("0");
            Some(pb)
        };

        let filename = single_output_filename(&config.input, &config.template, &template_extension);
        let output_path = PathBuf::from(&config.output).join(&filename);

        if config.verbose {
            eprintln!(
                "Processing {} entries -> {}",
                stats.entries_processed, filename
            );
        }

        if config.dry_run {
            println!("entries -> {}", filename);
            if let Some(pb) = &progress {
                pb.set_message(stats.files_generated.to_string());
                pb.inc(1);
                pb.finish_with_message(stats.files_generated.to_string());
            }
            return Ok(stats);
        }

        if output_path.exists() && !config.overwrite && !confirm_overwrite(&output_path)? {
            eprintln!("Warning: Skipped existing file: {}", output_path.display());
            if let Some(pb) = &progress {
                pb.set_message(stats.files_generated.to_string());
                pb.inc(1);
                pb.finish_with_message(stats.files_generated.to_string());
            }
            return Ok(stats);
        }

        let owned_entries = entries
            .iter()
            .map(|entry| (*entry).clone())
            .collect::<Vec<_>>();
        let rendered = template_engine
            .render_entries(template_name, &owned_entries)
            .with_context(|| {
                format!(
                    "Failed to render entries in single mode using template: {}",
                    template_name
                )
            })?;

        utils::safe_write(&output_path, rendered.as_bytes())
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

        stats.files_generated = 1;
        if let Some(pb) = &progress {
            pb.set_message(stats.files_generated.to_string());
            pb.inc(1);
            pb.finish_with_message(stats.files_generated.to_string());
        }
        return Ok(stats);
    }

    let progress = if config.verbose {
        None
    } else {
        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {wide_bar} {pos}/{len} entries | files generated: {msg}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-"),
        );
        pb.set_message("0");
        Some(pb)
    };

    // Sequential processing keeps output order predictable based on input order.
    for entry in entries {
        stats.entries_processed += 1;

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
            if let Some(pb) = &progress {
                pb.set_message(stats.files_generated.to_string());
                pb.inc(1);
            }
            continue;
        }

        if output_path.exists() && !config.overwrite && !confirm_overwrite(&output_path)? {
            eprintln!("Warning: Skipped existing file: {}", output_path.display());
            if let Some(pb) = &progress {
                pb.set_message(stats.files_generated.to_string());
                pb.inc(1);
            }
            continue;
        }

        let rendered = template_engine
            .render_entry(template_name, entry)
            .with_context(|| format!("Failed to render entry: {}", entry.key))?;

        utils::safe_write(&output_path, rendered.as_bytes())
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

        stats.files_generated += 1;

        if let Some(pb) = &progress {
            pb.set_message(stats.files_generated.to_string());
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_with_message(stats.files_generated.to_string());
    }

    Ok(stats)
}

fn single_output_filename(input_path: &str, template_path: &str, extension: &str) -> String {
    let input_stem = utils::stem(input_path).unwrap_or_else(|| "entries".to_string());
    let template_stem = utils::stem(template_path).unwrap_or_else(|| "output".to_string());
    format!("{}_{}.{}", input_stem, template_stem, extension)
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
        let has_explicit_selection =
            !config.filter.include.is_empty() || !config.filter.exclude.is_empty();
        let selected = entries
            .iter()
            .filter(|entry| config.filter.should_include_entry(&entry.key))
            .collect::<Vec<_>>();

        if has_explicit_selection && !selected.is_empty() {
            let mut by_key = BTreeMap::new();
            for entry in selected {
                by_key.insert(&entry.key, entry);
            }

            println!("{}", serde_json::to_string_pretty(&by_key)?);
            return Ok(());
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&entry_type_field_map(&entries))?
        );
        return Ok(());
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

fn entry_type_field_map(
    entries: &[bibtera::parser::BibTeXEntry],
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut fields_by_type = BTreeMap::<String, BTreeSet<String>>::new();

    for entry in entries {
        let fields = fields_by_type.entry(entry.entry_type.clone()).or_default();
        for field_name in entry.fields.keys() {
            fields.insert(field_name.clone());
        }
    }

    let mut map = BTreeMap::new();
    for (entry_type, fields) in fields_by_type {
        let field_refs = fields.iter().map(String::as_str).collect::<Vec<_>>();
        map.insert(entry_type, template_available_fields_for_type(&field_refs));
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
    map.insert(
        "slugified_keywords".to_string(),
        "array<string>".to_string(),
    );
    map.insert("fields".to_string(), "map<string,string>".to_string());
    map.insert("fields.month".to_string(), "string".to_string());

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

    #[test]
    fn test_entry_type_field_map_uses_present_types_and_fields() {
        let entries = vec![
            bibtera::parser::BibTeXEntry::new(
                "smith2020machine".to_string(),
                "article".to_string(),
                vec!["John Smith".to_string()],
                "Machine Learning".to_string(),
            )
            .with_field("journal".to_string(), "Journal of AI".to_string()),
            bibtera::parser::BibTeXEntry::new(
                "alice2022blog".to_string(),
                "misc".to_string(),
                vec!["Alice Johnson".to_string()],
                "Introduction to Rust".to_string(),
            )
            .with_field(
                "howpublished".to_string(),
                "https://example.com".to_string(),
            ),
        ];

        let map = entry_type_field_map(&entries);

        assert!(map.contains_key("article"));
        assert!(map.contains_key("misc"));
        assert!(!map.contains_key("book"));
        assert!(
            map.get("article")
                .expect("article map")
                .contains_key("fields.journal")
        );
        assert!(
            map.get("misc")
                .expect("misc map")
                .contains_key("fields.howpublished")
        );
    }

    #[test]
    fn test_single_output_filename_uses_input_and_template_stems() {
        let filename = single_output_filename(
            "examples/input_sample.bib",
            "templates/sample_template.md",
            "md",
        );
        assert_eq!(filename, "input_sample_sample_template.md");
    }

    #[test]
    fn test_single_output_filename_with_different_extensions() {
        let filename =
            single_output_filename("data/references.bib", "output/template.html", "html");
        assert_eq!(filename, "references_template.html");
    }

    #[test]
    fn test_single_output_filename_with_complex_paths() {
        let filename = single_output_filename(
            "src/bib/my_references.bib",
            "src/templates/my_template.json",
            "json",
        );
        assert_eq!(filename, "my_references_my_template.json");
    }

    #[test]
    fn test_single_output_filename_fallback_when_no_stems() {
        let filename = single_output_filename(".", ".", "md");
        assert_eq!(filename, "entries_output.md");
    }
}
