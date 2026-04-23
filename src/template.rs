//! Template rendering module.
//!
//! This module provides functionality to render BibTeX entries using Tera templates.
//! Each template can generate any text-based output format by specifying the appropriate
//! template file with the desired extension (e.g., template.md for Markdown, template.html for HTML).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tera::{Context as TeraContext, Tera};
use thiserror::Error;

use crate::parser::BibTeXEntry;

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

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Result<Self> {
        let tera = Tera::default();

        Ok(Self {
            tera,
            custom_templates: Vec::new(),
        })
    }

    /// Create a new template engine with custom templates from a directory
    pub fn from_directory<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Err(TemplateError::NotFound(format!(
                "Template directory does not exist: {}",
                dir.display()
            ))
            .into());
        }

        let tera = Tera::default();

        let mut engine = Self {
            tera,
            custom_templates: Vec::new(),
        };

        // Load all template files from the directory
        engine.load_templates_from_dir(dir)?;

        Ok(engine)
    }

    /// Add a custom template file
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
        let processed_template = preprocess_template_source(&template_source)
            .with_context(|| format!("Failed to preprocess template file '{}'", path.display()))?;

        self.tera
            .add_raw_template(&template_name, &processed_template)
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

    /// Load templates from a directory
    fn load_templates_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                self.add_template(&path)?;
            }
        }

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
        Self::new().unwrap_or_else(|_| Self {
            tera: Tera::default(),
            custom_templates: Vec::new(),
        })
    }
}

fn preprocess_template_source(source: &str) -> Result<String> {
    let mut result = String::with_capacity(source.len());
    let mut in_literal_block = false;

    for line in source.split_inclusive('\n') {
        let (line_body, line_ending) = split_line_ending(line);
        let trimmed = line_body.trim_start();
        let indentation = &line_body[..line_body.len() - trimmed.len()];

        if in_literal_block {
            if trimmed == "[/LITERAL]" {
                result.push_str("{% endraw %}");
                result.push_str(line_ending);
                in_literal_block = false;
            } else {
                result.push_str(line);
            }

            continue;
        }

        if trimmed == "[LITERAL]" {
            result.push_str(indentation);
            result.push_str("{% raw %}");
            result.push_str(line_ending);
            in_literal_block = true;
            continue;
        }

        if trimmed == "[/LITERAL]" {
            return Err(TemplateError::LoadError(
                "Encountered [/LITERAL] without a matching [LITERAL]".to_string(),
            )
            .into());
        }

        if let Some(literal) = trimmed.strip_prefix("LITERAL:") {
            result.push_str(indentation);
            result.push_str("{% raw %}");
            result.push_str(literal);
            result.push_str(line_ending);
            result.push_str("{% endraw %}");
            continue;
        }

        result.push_str(line);
    }

    if in_literal_block {
        return Err(TemplateError::LoadError(
            "Encountered [LITERAL] without a matching [/LITERAL]".to_string(),
        )
        .into());
    }

    Ok(result)
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(stripped) = line.strip_suffix("\r\n") {
        (stripped, "\r\n")
    } else if let Some(stripped) = line.strip_suffix('\n') {
        (stripped, "\n")
    } else {
        (line, "")
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
    fn test_render_entry_with_literal_line_marker() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_literal_line_template.md");
        let template_content = "LITERAL:{{ zola.title }}\nTitle: {{ title }}\n";
        std::fs::write(&temp_file, template_content).unwrap();

        engine.add_template(&temp_file).unwrap();

        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        );

        let rendered = engine
            .render_entry("test_literal_line_template", &entry)
            .unwrap();
        assert!(rendered.contains("{{ zola.title }}"));
        assert!(rendered.contains("Title: Test Title"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_render_entry_with_literal_block_markers() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_literal_block_template.md");
        let template_content = concat!(
            "[LITERAL]\n",
            "{% alert(type=\"info\", title=\"Author information\") %}\n",
            "{{ authors | join(sep=\"; \") }}\n",
            "{% end %}\n",
            "[/LITERAL]\n",
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
            .render_entry("test_literal_block_template", &entry)
            .unwrap();
        assert!(rendered.contains("{% alert(type=\"info\", title=\"Author information\") %}"));
        assert!(rendered.contains("{{ authors | join(sep=\"; \") }}"));
        assert!(rendered.contains("{% end %}"));
        assert!(rendered.contains("Title: Test Title"));

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_add_template_fails_for_unclosed_literal_block() {
        let mut engine = TemplateEngine::new().unwrap();
        let temp_file = std::env::temp_dir().join("test_literal_block_unclosed.md");
        std::fs::write(&temp_file, "[LITERAL]\n{{ downstream.syntax }}\n").unwrap();

        let error = engine.add_template(&temp_file).unwrap_err();
        let error_text = format!("{error:#}");
        assert!(error_text.contains("matching [/LITERAL]"));

        let _ = std::fs::remove_file(temp_file);
    }
}
