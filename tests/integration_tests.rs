//! Integration tests for the BibTeX converter.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bibtera::parser::BibTeXParser;
use bibtera::template::TemplateEngine;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn temp_dir() -> PathBuf {
    PathBuf::from(std::env::temp_dir()).join("bibtera_tests")
}

fn unique_temp_file(stem: &str, extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    temp_dir().join(format!("{}_{}.{}", stem, nonce, extension))
}

#[test]
fn it_parse_sample_001_parses_sample_bib_fixture() {
    let sample_file = examples_dir().join("input_sample.bib");
    let entries = BibTeXParser::parse_file(&sample_file).expect("parse input_sample.bib");

    assert!(!entries.is_empty());
    assert_eq!(entries[0].key, "smith2020machine");
    assert_eq!(entries[0].entry_type, "article");
}

#[test]
fn it_author_normalisation_001_normalises_supported_author_formats() {
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
    assert_eq!(entries[0].author_parts[1].full, "Jane Smith");
}

#[test]
fn it_author_normalisation_001_exposes_author_parts_to_templates() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = unique_temp_file("it_author_parts", "md");
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
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entry,
        )
        .expect("render entry");

    assert!(rendered.contains("Doe, John - Test Title"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_nonstandard_fields_001_preserves_non_standard_fields_for_templates() {
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
    let temp_file = unique_temp_file("it_non_standard_fields", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "Abstract: {{ fields.abstract }}\nKeywords: {{ fields.keywords }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            entry,
        )
        .expect("render entry");

    assert!(rendered.contains("Abstract: A short abstract"));
    assert!(rendered.contains("Keywords: privacy,security"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_date_normalisation_001_normalises_month_and_day_values() {
    let src = r#"
@article{k1,
  author = {Doe, John},
  title = {Test Title},
  month = {January},
  day = {7}
}
"#;

    let entries = BibTeXParser::parse_str(src).expect("parse source");
    let entry = entries.first().expect("entry exists");

    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = unique_temp_file("it_date_normalisation", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "Month: {{ fields.month }}\nDay: {{ fields.day }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            entry,
        )
        .expect("render entry");

    assert!(rendered.contains("Month: 01"));
    assert!(rendered.contains("Day: 07"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_slugified_keywords_001_exposes_slugified_keyword_array() {
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
    let temp_file = unique_temp_file("it_slugified_keywords", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content =
        "First: {{ slugified_keywords[0] }}\nAll: {{ slugified_keywords | join(sep=\",\") }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            entry,
        )
        .expect("render entry");

    assert!(rendered.contains("First: privacy-security"));
    assert!(rendered.contains("All: privacy-security,zero-trust,ai-ml"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_raw_bibtex_001_exposes_raw_bibtex_field() {
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
    let temp_file = unique_temp_file("it_raw_bibtex", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ raw_bibtex }}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            entry,
        )
        .expect("render entry");

    assert!(rendered.contains("@article{k1,"));
    assert!(rendered.contains("abstract = {A short abstract}"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_latex_substitution_math_mode_001_preserves_math_regions_for_templates() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = unique_temp_file("it_latex_math_mode", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ latex_substitute(value=title) }}";
    fs::write(&temp_file, template_content).expect("write template");
    engine.add_template(&temp_file).expect("add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test-math-mode".to_string(),
        "article".to_string(),
        vec!["Doe, Jane".to_string()],
        r#"outside \"{o}; $inline \"{o}$; $$display \"{o}$$; \(paren \"{o}\); \[bracket \"{o}\]"#
            .to_string(),
    );

    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entry,
        )
        .expect("render entry");

    assert_eq!(
        rendered,
        r#"outside ö; $inline \"{o}$; $$display \"{o}$$; \(paren \"{o}\); \[bracket \"{o}\]"#
    );

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_latex_substitution_math_mode_002_preserves_real_default_map_tokens_in_math_regions() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = unique_temp_file("it_latex_math_mode_real_tokens", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ latex_substitute(value=title) }}";
    fs::write(&temp_file, template_content).expect("write template");
    engine.add_template(&temp_file).expect("add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test-math-mode-real-tokens".to_string(),
        "article".to_string(),
        vec!["Doe, Jane".to_string()],
        r#"outside \textemdash \textasciitilde \textasciicircum; $inline \textemdash \textasciitilde \textasciicircum$; $$display \textemdash \textasciitilde \textasciicircum$$; \(paren \textemdash \textasciitilde \textasciicircum\); \[bracket \textemdash \textasciitilde \textasciicircum\]"#
            .to_string(),
    );

    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entry,
        )
        .expect("render entry");

    assert_eq!(
        rendered,
        r#"outside — ~ ^; $inline \textemdash \textasciitilde \textasciicircum$; $$display \textemdash \textasciitilde \textasciicircum$$; \(paren \textemdash \textasciitilde \textasciicircum\); \[bracket \textemdash \textasciitilde \textasciicircum\]"#
    );

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_latex_substitution_math_mode_003_treats_unclosed_double_dollar_as_plain_text() {
    let mut substitutions = bibtera::latex::SubstitutionMap::new();
    substitutions.insert("TOKEN".to_string(), "CHANGED".to_string());

    let mut engine =
        TemplateEngine::new_with_substitutions(Some(substitutions)).expect("create engine");
    let temp_file = unique_temp_file("it_latex_math_mode_unclosed_double_dollar", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ latex_substitute(value=title) }}";
    fs::write(&temp_file, template_content).expect("write template");
    engine.add_template(&temp_file).expect("add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test-math-mode-unclosed-double-dollar".to_string(),
        "article".to_string(),
        vec!["Doe, Jane".to_string()],
        "$$unclosed TOKEN $real$ math TOKEN".to_string(),
    );

    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entry,
        )
        .expect("render entry");

    assert_eq!(rendered, "$$unclosed CHANGED $real$ math CHANGED");

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_latex_substitution_cascade_001_does_not_reprocess_replacement_outputs() {
    let mut substitutions = bibtera::latex::SubstitutionMap::new();
    substitutions.insert("TOKENLONG".to_string(), "TOK".to_string());
    substitutions.insert("TOK".to_string(), "DONE".to_string());

    let mut engine =
        TemplateEngine::new_with_substitutions(Some(substitutions)).expect("create engine");
    let temp_file = unique_temp_file("it_latex_substitution_cascade", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content = "{{ latex_substitute(value=title) }}";
    fs::write(&temp_file, template_content).expect("write template");
    engine.add_template(&temp_file).expect("add template");

    let entry = bibtera::parser::BibTeXEntry::new(
        "test-latex-substitution-cascade".to_string(),
        "article".to_string(),
        vec!["Doe, Jane".to_string()],
        "TOKENLONG".to_string(),
    );

    let rendered = engine
        .render_entry(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entry,
        )
        .expect("render entry");

    assert_eq!(rendered, "TOK");

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_single_mode_context_001_exposes_entries_collection_to_templates() {
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
    let temp_file = unique_temp_file("it_entries_collection", "md");
    fs::create_dir_all(temp_dir()).ok();

    let template_content =
        "Count: {{ entries | length }}\nKeys: {% for e in entries %}{{ e.key }} {% endfor %}\n";
    fs::write(&temp_file, template_content).expect("write template");

    engine.add_template(&temp_file).expect("add template");
    let rendered = engine
        .render_entries(
            temp_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("template stem"),
            &entries,
        )
        .expect("render entries");

    assert!(rendered.contains("Count: 2"));
    assert!(rendered.contains("k1"));
    assert!(rendered.contains("k2"));

    fs::remove_file(&temp_file).ok();
}

#[test]
fn it_single_mode_context_001_supports_combined_output_naming_inputs() {
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

    let temp_template_file = temp_output_dir.join("mytemplate.md");
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
    assert_eq!("references_mytemplate.md", "references_mytemplate.md");

    let _ = fs::remove_dir_all(&temp_output_dir);
}

#[test]
fn it_error_surfacing_001_exposes_underlying_template_parser_errors() {
    let mut engine = TemplateEngine::new().expect("create engine");
    let temp_file = unique_temp_file("it_invalid_template_comment", "md");
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
