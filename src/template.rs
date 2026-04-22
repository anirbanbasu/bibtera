//! Template rendering module.
//!
//! This module provides functionality to render BibTeX entries using Tera templates.
//! It supports custom templates for different output formats (Markdown, HTML, JSON).

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

/// Output format handler for rendering entries
pub trait OutputHandler {
    /// Get the file extension for this format
    fn extension(&self) -> &'static str;

    /// Render a single entry to string
    fn render_entry(&self, entry: &BibTeXEntry) -> Result<String>;

    /// Render multiple entries to string
    fn render_entries(&self, entries: &[BibTeXEntry]) -> Result<String> {
        let mut results = Vec::new();
        for entry in entries {
            results.push(self.render_entry(entry)?);
        }
        Ok(results.join("\n\n"))
    }
}

/// Markdown output handler
pub struct MarkdownHandler;

impl MarkdownHandler {
    /// Create a new Markdown handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputHandler for MarkdownHandler {
    fn extension(&self) -> &'static str {
        "md"
    }

    fn render_entry(&self, entry: &BibTeXEntry) -> Result<String> {
        let mut output = String::new();

        // Add header with title
        output.push_str(&format!("# {}", entry.title));
        output.push('\n');
        output.push('\n');

        // Add citation key as comment
        output.push_str(&format!("<!-- citation: {} -->", entry.key));
        output.push('\n');
        output.push('\n');

        // Add authors
        if !entry.authors.is_empty() {
            output.push_str("**Authors**: ");
            output.push_str(&entry.authors.join(", "));
            output.push('\n');
            output.push('\n');
        }

        // Add year
        if let Some(year) = &entry.year {
            output.push_str("**Year**: ");
            output.push_str(year);
            output.push('\n');
            output.push('\n');
        }

        // Add entry type
        output.push_str("**Type**: ");
        output.push_str(&entry.entry_type);
        output.push('\n');
        output.push('\n');

        // Add additional fields
        if !entry.fields.is_empty() {
            output.push_str("**Fields**:\n\n");

            for (key, value) in &entry.fields {
                output.push_str(&format!("- **{}**: {}", key, value));
                output.push('\n');
            }
            output.push('\n');
        }

        Ok(output)
    }
}

/// HTML output handler
pub struct HtmlHandler;

impl HtmlHandler {
    /// Create a new HTML handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for HtmlHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputHandler for HtmlHandler {
    fn extension(&self) -> &'static str {
        "html"
    }

    fn render_entry(&self, entry: &BibTeXEntry) -> Result<String> {
        let mut output = String::new();

        // Escape HTML entities in strings
        let escape = |s: &str| -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;")
        };

        // Add header with title
        output.push_str(&format!("<h1>{}</h1>", escape(&entry.title)));
        output.push('\n');

        // Add authors
        if !entry.authors.is_empty() {
            output.push_str("<p><strong>Authors</strong>: ");
            output.push_str(&entry.authors.join(", "));
            output.push_str("</p>\n");
        }

        // Add year
        if let Some(year) = &entry.year {
            output.push_str("<p><strong>Year</strong>: ");
            output.push_str(year);
            output.push_str("</p>\n");
        }

        // Add entry type
        output.push_str("<p><strong>Type</strong>: ");
        output.push_str(&entry.entry_type);
        output.push_str("</p>\n");

        // Add additional fields
        if !entry.fields.is_empty() {
            output.push_str("<dl>\n");
            for (key, value) in &entry.fields {
                output.push_str(&format!(
                    "<dt><strong>{}</strong></dt><dd>{}</dd>\n",
                    escape(key),
                    escape(value)
                ));
            }
            output.push_str("</dl>\n");
        }

        Ok(output)
    }
}

/// JSON output handler
pub struct JsonHandler;

impl JsonHandler {
    /// Create a new JSON handler
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputHandler for JsonHandler {
    fn extension(&self) -> &'static str {
        "json"
    }

    fn render_entry(&self, entry: &BibTeXEntry) -> Result<String> {
        use serde_json::json;

        let json_obj = json!({
            "key": entry.key,
            "entry_type": entry.entry_type,
            "authors": entry.authors,
            "title": entry.title,
            "year": entry.year,
            "fields": entry.fields
        });

        Ok(serde_json::to_string_pretty(&json_obj)?)
    }
}

/// Template engine wrapper
pub struct TemplateEngine {
    /// Tera instance
    tera: Tera,

    /// Custom template paths
    custom_templates: Vec<PathBuf>,
}

impl TemplateEngine {
    /// Create a new template engine with default templates
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Load built-in templates
        tera.add_raw_template(
            "bibtex_entry.md",
            include_str!("../templates/bibtex_entry.md"),
        )
        .context("Failed to load default markdown template")?;
        tera.add_raw_template(
            "bibtex_entry.html",
            include_str!("../templates/bibtex_entry.html"),
        )
        .context("Failed to load default html template")?;
        tera.add_raw_template(
            "bibtex_entry.json",
            include_str!("../templates/bibtex_entry.json"),
        )
        .context("Failed to load default json template")?;

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

        // Load all .tera files from the directory
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

        self.tera
            .add_template_file(path, Some(&template_name))
            .context("Failed to load template")?;

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

            if path.is_file() && path.extension().is_some_and(|ext| ext == "tera") {
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

        // Add year (optionally)
        if let Some(year) = &entry.year {
            context.insert("year", year);
        }

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

/// Render entries using built-in output handlers
pub fn render_with_handler<P: AsRef<Path>>(
    handler: &dyn OutputHandler,
    entry: &BibTeXEntry,
    output_path: P,
) -> Result<()> {
    let output_path = output_path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    // Render and write
    let rendered = handler.render_entry(entry)?;

    fs::write(output_path, &rendered).context("Failed to write output file")?;

    Ok(())
}

/// Render multiple entries using built-in output handlers
pub fn render_entries_with_handler<P: AsRef<Path>>(
    handler: &dyn OutputHandler,
    entries: &[BibTeXEntry],
    output_path: P,
) -> Result<()> {
    let output_path = output_path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    // Render and write
    let rendered = handler.render_entries(entries)?;

    fs::write(output_path, &rendered).context("Failed to write output file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_handler() {
        let handler = MarkdownHandler::new();
        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        );

        let rendered = handler.render_entry(&entry).unwrap();

        assert!(rendered.contains("# Test Title"));
        assert!(rendered.contains("Author One"));
        assert!(rendered.contains("article"));
    }

    #[test]
    fn test_html_handler() {
        let handler = HtmlHandler::new();
        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "book".to_string(),
            vec!["Author Two".to_string()],
            "Another Title".to_string(),
        );

        let rendered = handler.render_entry(&entry).unwrap();

        assert!(rendered.contains("<h1>Another Title</h1>"));
        assert!(rendered.contains("Author Two"));
        assert!(rendered.contains("<strong>Type</strong>: book"));
    }

    #[test]
    fn test_json_handler() {
        let handler = JsonHandler::new();
        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "inproceedings".to_string(),
            vec!["Author Three".to_string()],
            "Conference Paper".to_string(),
        );

        let rendered = handler.render_entry(&entry).unwrap();

        assert!(rendered.contains("\"key\": \"test2024\""));
        assert!(rendered.contains("\"entry_type\": \"inproceedings\""));
        assert!(rendered.contains("Conference Paper"));
    }
}
