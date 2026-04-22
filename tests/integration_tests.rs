//! Integration tests for the BibTeX converter.
//!
//! These tests verify the end-to-end functionality of the application,
//! including parsing BibTeX files and rendering them to Tera templates.

use std::fs;
use std::path::PathBuf;

use bibtera::config::Config;
use bibtera::parser::BibTeXParser;
use bibtera::template::TemplateEngine;

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
fn test_template_engine_rendering() {
    let mut engine = TemplateEngine::new().expect("Failed to create template engine");

    // Create a temporary template file
    let temp_file = temp_dir().join("test_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "# {{ title }}\n\nKey: {{ key }}\n\nAuthors: {% for author in authors %}{{ author }}{% if not loop.last %}, {% endif %}{% endfor %}";
    fs::write(&temp_file, template_content).expect("Failed to write template");

    engine
        .add_template(&temp_file)
        .expect("Failed to add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test2024".to_string(),
        "article".to_string(),
        vec!["Author One".to_string(), "Author Two".to_string()],
        "Test Title".to_string(),
    );

    let rendered = engine
        .render_entry("test_template", &entry)
        .expect("Failed to render");

    assert!(rendered.contains("# Test Title"));
    assert!(rendered.contains("Key: test2024"));
    assert!(rendered.contains("Author One"));

    // Cleanup
    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_full_workflow() {
    let sample_file = examples_dir().join("sample.bib");

    // Parse
    let entries = BibTeXParser::parse_file(&sample_file).expect("Failed to parse");
    assert!(!entries.is_empty());
}

#[test]
fn test_config_creation() {
    let config = Config {
        input: Some("test.bib".to_string()),
        output: Some("output".to_string()),
        template: Some("template.md".to_string()),
        exclude: vec![],
        include: vec![],
        dry_run: false,
        overwrite: false,
        verbose: false,
    };

    assert_eq!(config.input, Some("test.bib".to_string()));
    assert_eq!(config.output, Some("output".to_string()));
    assert_eq!(config.template, Some("template.md".to_string()));
    assert!(!config.dry_run);
    assert!(!config.overwrite);
}

#[test]
fn test_config_from_cli() {
    let config = Config::from_cli(
        Some("test.bib".to_string()),
        Some("output".to_string()),
        Some("template.md".to_string()),
        None,
        None,
        false,
        false,
        false,
    )
    .expect("Failed to create config from CLI");

    assert_eq!(config.input, Some("test.bib".to_string()));
    assert_eq!(config.output, Some("output".to_string()));
    assert_eq!(config.template, Some("template.md".to_string()));
}

#[test]
fn test_config_exclude_and_include_mutually_exclusive() {
    let result = Config::from_cli(
        Some("test.bib".to_string()),
        Some("output".to_string()),
        Some("template.md".to_string()),
        Some("key1,key2".to_string()),
        Some("key3,key4".to_string()),
        false,
        false,
        false,
    );

    assert!(result.is_err());
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
fn test_should_include_entry_with_include() {
    let config = Config {
        include: vec!["key1".to_string(), "key2".to_string()],
        ..Default::default()
    };

    assert!(config.should_include_entry("key1"));
    assert!(config.should_include_entry("key2"));
    assert!(!config.should_include_entry("key3"));
}

#[test]
fn test_should_include_entry_with_exclude() {
    let config = Config {
        exclude: vec!["key1".to_string()],
        ..Default::default()
    };

    assert!(!config.should_include_entry("key1"));
    assert!(config.should_include_entry("key2"));
    assert!(config.should_include_entry("key3"));
}

#[test]
fn test_should_include_entry_no_filters() {
    let config = Config::default();

    assert!(config.should_include_entry("key1"));
    assert!(config.should_include_entry("key2"));
    assert!(config.should_include_entry("any_key"));
}
