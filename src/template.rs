//! Template rendering module.
//!
//! This module provides functionality to render BibTeX entries using Tera templates.
//! Each template can generate any text-based output format by specifying the appropriate
//! template file with the desired extension (e.g., template.md for Markdown, template.html for HTML).

#[cfg(test)]
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tera::{
    Context as TeraContext, Error as TeraError, Kwargs, State as TeraState, Tera, TeraResult, Value,
};
use thiserror::Error;

use crate::latex::{
    SubstitutionMap, build_substitution_map, ordered_substitutions,
    substitute_latex_to_text_with_ordered,
};
use crate::parser::BibTeXEntry;

fn extract_substitution_input(kwargs: &Kwargs) -> TeraResult<String> {
    for key in ["value", "text", "input"] {
        if let Some(value) = kwargs.get::<&str>(key)? {
            return Ok(value.to_string());
        }
    }

    Err(TeraError::message(
        "latex_substitute expects one string argument via `value`, `text`, or `input`",
    ))
}

/// JSON-encode a value, mirroring the `json_encode` filter that shipped as a
/// Tera built-in prior to Tera 2.0. Templates such as
/// `examples/template_entry_single.json` rely on this filter to escape field
/// values for embedding in JSON output.
fn json_encode_filter(value: &Value, kwargs: Kwargs, _state: &TeraState) -> TeraResult<String> {
    let pretty = kwargs.get::<bool>("pretty")?.unwrap_or(false);
    let encoded = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    encoded.map_err(|error| TeraError::message(error.to_string()))
}

/// Error types for template rendering operations
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Error when loading templates
    #[error("Failed to load templates: {0}")]
    LoadError(String),

    /// Error when rendering a template
    #[error("Failed to render template: {0}")]
    RenderError(String),

    /// Error when writing output file
    #[error("Failed to write output file: {0}")]
    WriteError(String),

    /// Error when template file not found
    #[error("Template file not found: {0}")]
    NotFound(String),
}

/// Template engine for rendering BibTeX entries with Tera templates
pub struct TemplateEngine {
    /// Tera instance
    tera: Tera,

    /// Custom template paths
    custom_templates: Vec<PathBuf>,
}

#[cfg(test)]
thread_local! {
    static FORCE_TEMPLATE_ENGINE_INIT_FAILURE: Cell<bool> = const { Cell::new(false) };
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Result<Self> {
        Self::new_with_substitutions(None)
    }

    /// Create a new template engine with optional custom substitution overrides
    pub fn new_with_substitutions(custom_substitutions: Option<SubstitutionMap>) -> Result<Self> {
        #[cfg(test)]
        if FORCE_TEMPLATE_ENGINE_INIT_FAILURE.with(|should_fail| should_fail.get()) {
            anyhow::bail!("Forced template engine initialisation failure for tests");
        }

        let mut tera = Tera::default();
        let substitutions = build_substitution_map(custom_substitutions)?;
        let ordered = ordered_substitutions(&substitutions);

        let function_substitutions = ordered.clone();
        tera.register_function(
            "latex_substitute",
            move |kwargs: Kwargs, _state: &TeraState| -> TeraResult<Value> {
                let input = extract_substitution_input(&kwargs)?;
                let substituted =
                    substitute_latex_to_text_with_ordered(&input, &function_substitutions);
                Ok(Value::from(substituted))
            },
        );

        tera.register_filter(
            "latex_substitute",
            move |value: &str, _kwargs: Kwargs, _state: &TeraState| -> String {
                substitute_latex_to_text_with_ordered(value, &ordered)
            },
        );

        tera.register_filter("json_encode", json_encode_filter);

        Ok(Self {
            tera,
            custom_templates: Vec::new(),
        })
    }

    /// Add a custom template file
    ///
    /// Templates are registered with Tera under their file stem, without the
    /// original extension, so Tera's built-in HTML autoescaping never
    /// activates regardless of the template's output format. This is
    /// deliberate: BibTera's sole intended output is non-HTML text, such as
    /// Markdown, which may itself embed further Tera-like markup destined for
    /// a downstream processor such as Zola (see FUNC-6, FUNC-6.1 in
    /// `REQUIREMENTS.md`). Autoescaping would corrupt that downstream markup
    /// as well as legitimate LaTeX control characters (`\`, `{`, `}`) in
    /// rendered content. Authors of templates that do produce HTML are
    /// responsible for escaping HTML-significant characters themselves.
    pub fn add_template<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        let template_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                TemplateError::NotFound(format!("Invalid template file name: {}", path.display()))
            })?
            .to_string();

        let template_source = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file '{}'", path.display()))?;

        self.tera
            .add_raw_template(&template_name, &template_source)
            .with_context(|| {
                format!(
                    "Failed to load template file '{}' as template '{}'",
                    path.display(),
                    template_name
                )
            })?;

        self.custom_templates.push(path.to_path_buf());

        Ok(())
    }

    /// Render a single entry using a template
    pub fn render_entry(&self, template_name: &str, entry: &BibTeXEntry) -> Result<String> {
        let mut context = TeraContext::new();

        // Add entry data to context
        context.insert("key", &entry.key);
        context.insert("entry_type", &entry.entry_type);
        context.insert("title", &entry.title);

        // Add authors as array
        let authors: Vec<&str> = entry.authors.iter().map(|a| a.as_str()).collect();
        context.insert("authors", &authors);

        // Add structured author parts for advanced templates
        context.insert("author_parts", &entry.author_parts);

        // Add year (if present)
        if let Some(year) = &entry.year {
            context.insert("year", year);
        }

        // Add raw BibTeX representation
        context.insert("raw_bibtex", &entry.raw_bibtex);

        // Add slugified keywords from the BibTeX keywords field
        context.insert("slugified_keywords", &entry.slugified_keywords);

        // Add fields as object
        let fields: HashMap<&str, &str> = entry
            .fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        context.insert("fields", &fields);

        // Render template
        let rendered = self
            .tera
            .render(template_name, &context)
            .map_err(|e| TemplateError::RenderError(e.to_string()))?;

        Ok(rendered)
    }

    /// Render multiple entries using a template
    pub fn render_entries(&self, template_name: &str, entries: &[BibTeXEntry]) -> Result<String> {
        let mut context = TeraContext::new();

        // Add entries as array
        let entries_data: Vec<&BibTeXEntry> = entries.iter().collect();
        context.insert("entries", &entries_data);

        // Render template
        let rendered = self
            .tera
            .render(template_name, &context)
            .map_err(|e| TemplateError::RenderError(e.to_string()))?;

        Ok(rendered)
    }

    /// Get the tera instance for advanced usage
    pub fn get_tera(&self) -> &Tera {
        &self.tera
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
            .unwrap_or_else(|error| panic!("Failed to initialise template engine: {error:#}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_creation() {
        let engine = TemplateEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_add_template() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_template.md");
        std::fs::write(&temp_file, "# {{ title }}\n{{ key }}").unwrap();

        let result = engine.add_template(&temp_file);
        assert!(result.is_ok());

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_template() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_render_template.md");
        let template_content = "Title: {{ title }}\nKey: {{ key }}";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        );

        let rendered = engine.render_entry("test_render_template", &entry);
        assert!(rendered.is_ok());
        let rendered_str = rendered.unwrap();
        assert!(rendered_str.contains("Title: Test Title"));
        assert!(rendered_str.contains("Key: test2024"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_tera_raw_line() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_tera_raw_line_template.md");
        let template_content = "{% raw %}{{ zola.title }}{% endraw %}\nTitle: {{ title }}\n";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        );

        let rendered = engine
            .render_entry("test_tera_raw_line_template", &entry)
            .unwrap();
        assert!(rendered.contains("{{ zola.title }}"));
        assert!(rendered.contains("Title: Test Title"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_tera_raw_block() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_tera_raw_block_template.md");
        let template_content = concat!(
            "{% raw %}\n",
            "{% alert(type=\"info\", title=\"Author information\") %}\n",
            "{{ authors | join(sep=\"; \") }}\n",
            "{% end %}\n",
            "{% endraw %}\n",
            "Title: {{ title }}\n"
        );
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        );

        let rendered = engine
            .render_entry("test_tera_raw_block_template", &entry)
            .unwrap();
        assert!(rendered.contains("{% alert(type=\"info\", title=\"Author information\") %}"));
        assert!(rendered.contains("{{ authors | join(sep=\"; \") }}"));
        assert!(rendered.contains("{% end %}"));
        assert!(rendered.contains("Title: Test Title"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_add_template_fails_for_unclosed_tera_raw_block() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_tera_raw_block_unclosed.md");
        std::fs::write(&temp_file, "{% raw %}\n{{ downstream.syntax }}\n").unwrap();

        let error = engine.add_template(&temp_file).unwrap_err();
        let error_text = format!("{error:#}");
        assert!(error_text.contains("Failed to load template file"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_latex_substitute_function() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_latex_substitute_function.md");
        let template_content = "{{ latex_substitute(value=title) }}";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "\\textbf{G\\\"{o}del and \\emph{Br\\'{e}zis}}".to_string(),
        );

        let rendered = engine
            .render_entry("test_latex_substitute_function", &entry)
            .unwrap();
        assert_eq!(rendered, "Gödel and Brézis");

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_latex_substitute_filter() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_latex_substitute_filter.md");
        let template_content = "{{ title | latex_substitute }}";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "\\textit{Wei\\ss and H\\\"{o}lder}".to_string(),
        );

        let rendered = engine
            .render_entry("test_latex_substitute_filter", &entry)
            .unwrap();
        assert_eq!(rendered, "Weiß and Hölder");

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_custom_substitution_override() {
        let mut custom = SubstitutionMap::new();
        custom.insert("\\textemdash".to_string(), "--".to_string());

        let mut engine = TemplateEngine::new_with_substitutions(Some(custom)).unwrap();
        let temp_file = std::env::temp_dir().join("test_latex_substitute_override.md");
        let template_content = "{{ latex_substitute(value=title) }}";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "A \\textemdash B".to_string(),
        );

        let rendered = engine
            .render_entry("test_latex_substitute_override", &entry)
            .unwrap();
        assert_eq!(rendered, "A -- B");

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_default_template_engine_registers_latex_substitute_helpers() {
        let mut engine = TemplateEngine::default();
        let temp_file = std::env::temp_dir().join("test_default_engine_latex_substitute.md");
        let template_content =
            "{{ latex_substitute(value=title) }} :: {{ title | latex_substitute }}";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "A \\textemdash B".to_string(),
        );

        let rendered = engine
            .render_entry("test_default_engine_latex_substitute", &entry)
            .unwrap();
        assert_eq!(rendered, "A — B :: A — B");

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_default_template_engine_panics_when_initialisation_fails() {
        struct ForceInitFailureReset;
        impl Drop for ForceInitFailureReset {
            fn drop(&mut self) {
                FORCE_TEMPLATE_ENGINE_INIT_FAILURE.with(|should_fail| should_fail.set(false));
            }
        }

        FORCE_TEMPLATE_ENGINE_INIT_FAILURE.with(|should_fail| should_fail.set(true));
        let _reset = ForceInitFailureReset;
        let panic_result = std::panic::catch_unwind(TemplateEngine::default);

        let panic_payload = match panic_result {
            Ok(_) => panic!("default constructor should panic"),
            Err(payload) => payload,
        };
        let panic_message = if let Some(message) = panic_payload.downcast_ref::<String>() {
            message.clone()
        } else if let Some(message) = panic_payload.downcast_ref::<&str>() {
            message.to_string()
        } else {
            String::new()
        };

        assert!(panic_message.contains("Failed to initialise template engine"));
        assert!(panic_message.contains("Forced template engine initialisation failure for tests"));
    }
}
