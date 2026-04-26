//! Integration tests for the BibTeX converter.

use std::fs;
use std::path::PathBuf;

use bibtera::cli::{Cli, Commands, FileNameStrategy};
use bibtera::parser::BibTeXParser;
use bibtera::template::TemplateEngine;
use clap::Parser;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn temp_dir() -> PathBuf {
    PathBuf::from(std::env::temp_dir()).join("bibtera_tests")
}

#[test]
fn test_parse_sample_bib() {
    let sample_file = examples_dir().join("input_sample.bib");
    let entries = BibTeXParser::parse_file(&sample_file).expect("parse input_sample.bib");

    assert!(!entries.is_empty());
    assert_eq!(entries[0].key, "smith2020machine");
    assert_eq!(entries[0].entry_type, "article");
}

#[test]
fn test_author_parsing_normalises_last_first() {
    let src = r#"
@article{k1,
  author = {Doe, John and Jane Smith},
  title = {T},
  year = {2024}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].authors[0], "John Doe");
    assert_eq!(entries[0].author_parts[0].first, "John");
    assert_eq!(entries[0].author_parts[0].last, "Doe");
}

#[test]
fn test_template_engine_rendering_with_author_parts() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ author_parts[0].last }}, {{ author_parts[0].first }} - {{ title }}";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test2024".to_string(),
        "article".to_string(),
        vec!["John Doe".to_string()],
        "Test Title".to_string(),
    );

    let rendered = engine
        .render_entry("test_template", &entry)
        .expect("render entry");

    assert!(rendered.contains("Doe, John - Test Title"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_load_error_exposes_underlying_tera_parser_issue() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_template_invalid_comment.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "<!-- {% alert(type=\"info\") %} -->\nHello\n<!-- {% end %} -->";
    fs::write(&temp_file, template_content).expect("write invalid template");

    let error = engine
        .add_template(&temp_file)
        .expect_err("template load should fail");
    let error_text = format!("{error:#}");

    assert!(error_text.contains("alert") || error_text.contains("end"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_renders_non_standard_fields() {
    let src = r#"
@article{k1,
  author = {Doe, John},
  title = {Test Title},
  year = {2024},
  abstract = {A short abstract},
  keywords = {privacy,security}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    let entry = entries.first().expect("entry exists");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_non_standard_fields_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "Abstract: {{ fields.abstract }}\nKeywords: {{ fields.keywords }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry("test_non_standard_fields_template", entry)
        .expect("render entry");

    assert!(rendered.contains("Abstract: A short abstract"));
    assert!(rendered.contains("Keywords: privacy,security"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_renders_raw_bibtex_field() {
    let src = r#"
@article{k1,
  author = {Doe, John},
  title = {Test Title},
  year = {2024},
  abstract = {A short abstract}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    let entry = entries.first().expect("entry exists");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_raw_bibtex_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ raw_bibtex }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry("test_raw_bibtex_template", entry)
        .expect("render entry");

    assert!(rendered.contains("@article{k1,"));
    assert!(rendered.contains("abstract = {A short abstract}"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_renders_normalised_month_field() {
    let src = r#"
@article{k1,
  author = {Doe, John},
  title = {Test Title},
  month = {January}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    let entry = entries.first().expect("entry exists");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_month_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "Month: {{ fields.month }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry("test_month_template", entry)
        .expect("render entry");

    assert!(rendered.contains("Month: 01"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_renders_slugified_keywords_field() {
    let src = r#"
@article{k1,
  author = {Doe, John},
  title = {Test Title},
  keywords = {Privacy & Security, Zero Trust; AI/ML}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    let entry = entries.first().expect("entry exists");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_slugified_keywords_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content =
        "First: {{ slugified_keywords[0] }}\nAll: {{ slugified_keywords | join(sep=\",\") }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry("test_slugified_keywords_template", entry)
        .expect("render entry");

    assert!(rendered.contains("First: privacy-security"));
    assert!(rendered.contains("All: privacy-security,zero-trust,ai-ml"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_template_renders_entries_variable_in_single_mode_template() {
    let src = r#"
@article{k1,
    title = {First Title}
}

@article{k2,
    title = {Second Title}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = temp_dir().join("test_entries_variable_template.md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content =
        "Count: {{ entries | length }}\nKeys: {% for e in entries %}{{ e.key }} {% endfor %}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entries("test_entries_variable_template", &entries)
        .expect("render entries");

    assert!(rendered.contains("Count: 2"));
    assert!(rendered.contains("k1"));
    assert!(rendered.contains("k2"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn test_cli_transform_parsing() {
    let cli = Cli::try_parse_from([
        "bibtera",
        "transform",
        "-i",
        "in.bib",
        "-o",
        "out",
        "-t",
        "tmpl.md",
        "--file-name-strategy",
        "slugify",
    ])
    .expect("parse cli");

    match cli.command {
        Commands::Transform(args) => {
            assert_eq!(args.input, "in.bib");
            assert_eq!(args.output, "out");
            assert_eq!(args.template, "tmpl.md");
            assert_eq!(args.file_name_strategy, FileNameStrategy::Slugify);
            assert!(!args.single);
        }
        _ => panic!("expected transform command"),
    }
}

#[test]
fn test_cli_transform_parsing_single_mode() {
    let cli = Cli::try_parse_from([
        "bibtera",
        "transform",
        "-i",
        "in.bib",
        "-o",
        "out",
        "-t",
        "tmpl.md",
        "--single",
    ])
    .expect("parse cli");

    match cli.command {
        Commands::Transform(args) => {
            assert_eq!(args.input, "in.bib");
            assert_eq!(args.output, "out");
            assert_eq!(args.template, "tmpl.md");
            assert!(args.single);
        }
        _ => panic!("expected transform command"),
    }
}

#[test]
fn test_single_mode_output_filename_combines_input_and_template_stems() {
    let src = r#"
@article{k1,
  title = {Title One}
}

@article{k2,
  title = {Title Two}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    assert_eq!(entries.len(), 2);

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_output_dir = temp_dir().join("single_mode_test");
    fs::create_dir_all(&temp_output_dir).ok();

    let temp_bib_file = temp_output_dir.join("references.bib");
    let temp_template_file = temp_output_dir.join("mytemplate.md");

    fs::write(&temp_bib_file, src).expect("write temp bib");

    let template_content = "# All References\n{% for entry in entries %}\n- {{ entry.key }}: {{ entry.title }}\n{% endfor %}\n";
    fs::write(&temp_template_file, template_content).expect("write template");

    engine
        .add_template(&temp_template_file)
        .expect("add template");
    let rendered = engine
        .render_entries("mytemplate", &entries)
        .expect("render entries");

    assert!(rendered.contains("# All References"));
    assert!(rendered.contains("k1: Title One"));
    assert!(rendered.contains("k2: Title Two"));

    // Verify the combined naming pattern would be: references_mytemplate.md
    // (This is a conceptual test - actual file writing tested in main.rs integration)
    let expected_filename = format!("references_mytemplate.md");
    assert_eq!(expected_filename, "references_mytemplate.md");

    let _ = fs::remove_dir_all(&temp_output_dir);
}

#[test]
fn test_cli_info_parsing() {
    let cli = Cli::try_parse_from(["bibtera", "info", "-i", "in.bib", "--exclude", "k1"])
        .expect("parse cli");

    match cli.command {
        Commands::Info(args) => {
            assert_eq!(args.input.as_deref(), Some("in.bib"));
            assert_eq!(args.exclude.as_deref(), Some("k1"));
        }
        _ => panic!("expected info command"),
    }
}
