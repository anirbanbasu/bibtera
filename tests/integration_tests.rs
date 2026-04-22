//! Integration tests for the BibTeX converter.
//!
//! These tests verify the end-to-end functionality of the application,
//! including parsing BibTeX files and rendering them to various formats.

use std::fs;
use std::path::PathBuf;

use bibtera::config::{Config, OutputFormat};
use bibtera::parser::BibTeXParser;
use bibtera::template::{HtmlHandler, JsonHandler, MarkdownHandler, OutputHandler};

/// Get the examples directory path
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// Get a temporary directory for test outputs
fn temp_dir() -> PathBuf {
    PathBuf::from(std::env::temp_dir()).join("bibtera_tests")
}

#[test]
fn test_parse_sample_bib() {
    let sample_file = examples_dir().join("sample.bib");
    assert!(sample_file.exists());

    let entries = BibTeXParser::parse_file(&sample_file).expect("Failed to parse sample.bib");

    assert!(!entries.is_empty());
    assert!(entries.len() >= 6);

    // Verify first entry
    let first = &entries[0];
    assert_eq!(first.key, "smith2020machine");
    assert_eq!(first.entry_type, "article");
    assert!(first.authors.contains(&"John Smith".to_string()));
    assert_eq!(
        first.title,
        "Machine Learning for Natural Language Processing"
    );
}

#[test]
fn test_parse_single_entry() {
    let entry = r#"
@article{test2024,
    author = {Test Author},
    title = {Test Title},
    year = {2024}
}
"#;

    let entries = BibTeXParser::parse_str(entry).expect("Failed to parse single entry");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "test2024");
    assert_eq!(entries[0].title, "Test Title");
}

#[test]
fn test_markdown_rendering() {
    let entries = vec![bibtera::parser::BibTeXEntry::new(
        "test1".to_string(),
        "article".to_string(),
        vec!["Author One".to_string()],
        "Test Title".to_string(),
    )];

    let handler = MarkdownHandler::new();
    let rendered = handler
        .render_entries(&entries)
        .expect("Failed to render markdown");

    assert!(rendered.contains("# Test Title"));
    assert!(rendered.contains("Author One"));
}

#[test]
fn test_html_rendering() {
    let entries = vec![bibtera::parser::BibTeXEntry::new(
        "test1".to_string(),
        "book".to_string(),
        vec!["Author Two".to_string()],
        "Book Title".to_string(),
    )];

    let handler = HtmlHandler::new();
    let rendered = handler
        .render_entries(&entries)
        .expect("Failed to render html");

    assert!(rendered.contains("<h1>Book Title</h1>"));
    assert!(rendered.contains("<strong>Authors</strong>"));
}

#[test]
fn test_json_rendering() {
    let entries = vec![bibtera::parser::BibTeXEntry::new(
        "test1".to_string(),
        "inproceedings".to_string(),
        vec!["Author Three".to_string()],
        "Conference Paper".to_string(),
    )];

    let handler = JsonHandler::new();
    let rendered = handler
        .render_entries(&entries)
        .expect("Failed to render json");

    assert!(rendered.contains("\"key\": \"test1\""));
    assert!(rendered.contains("\"entry_type\": \"inproceedings\""));
}

#[test]
fn test_full_workflow() {
    let sample_file = examples_dir().join("sample.bib");

    // Parse
    let entries = BibTeXParser::parse_file(&sample_file).expect("Failed to parse");

    // Render to Markdown
    let handler = MarkdownHandler::new();
    let rendered = handler.render_entries(&entries).expect("Failed to render");

    // Write to temp file
    let temp_output = temp_dir().join("test_output.md");
    fs::create_dir_all(temp_output.parent().unwrap()).ok();
    fs::write(&temp_output, &rendered).expect("Failed to write output");

    // Verify output exists
    assert!(temp_output.exists());

    // Cleanup
    fs::remove_file(&temp_output).ok();
}

#[test]
fn test_cli_config() {
    let config = Config::from_cli(
        Some("test.bib".to_string()),
        Some("output.md".to_string()),
        None,
        bibtera::cli::OutputFormat::Markdown,
        true,
        true,
    );

    assert_eq!(config.input, Some("test.bib".to_string()));
    assert_eq!(config.output, Some("output.md".to_string()));
    assert_eq!(config.format, OutputFormat::Markdown);
    assert!(config.recursive);
    assert!(config.verbose);
}

#[test]
fn test_directory_parsing() {
    let sample_dir = examples_dir();

    let entries = BibTeXParser::parse_directory(&sample_dir, true);

    assert!(entries.is_ok());
    let entries = entries.unwrap();
    assert!(!entries.is_empty());
}

#[test]
fn test_output_extension() {
    let config = Config::default();

    assert_eq!(config.output_extension(), "md");

    let mut config_html = config.clone();
    config_html.format = OutputFormat::Html;
    assert_eq!(config_html.output_extension(), "html");

    let mut config_json = config;
    config_json.format = OutputFormat::Json;
    assert_eq!(config_json.output_extension(), "json");
}

#[test]
fn test_output_filename() {
    let config = Config::default();

    assert_eq!(config.output_filename("paper.bib"), "paper.md");
    assert_eq!(config.output_filename("citation.bib"), "citation.md");
    assert_eq!(config.output_filename("test"), "test.md");
}
