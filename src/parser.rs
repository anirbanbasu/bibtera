//! BibTeX parsing module.
//!
//! This module provides functionality to parse BibTeX files using the BibLatex library.
//! It extracts entry types, keys, authors, titles, and other metadata from BibTeX entries.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use biblatex::{Bibliography, ChunksExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types for BibTeX parsing operations
#[derive(Error, Debug)]
pub enum ParseError {
    /// Error when reading the BibTeX file
    #[error("Failed to read file: {0}")]
    ReadError(#[from] std::io::Error),

    /// Error when parsing BibTeX content
    #[error("Failed to parse BibTeX content: {0}")]
    Parse(String),

    /// Error when no entries found in file
    #[error("No BibTeX entries found in file: {0}")]
    NoEntries(String),

    /// Error when processing directory
    #[error("Failed to process directory: {0}")]
    DirectoryError(String),
}

/// Represents a single BibTeX entry with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibTeXEntry {
    /// The citation key (e.g., "smith2020machine")
    pub key: String,

    /// The entry type (article, book, inproceedings, etc.)
    pub entry_type: String,

    /// Authors/creators of the work
    pub authors: Vec<String>,

    /// Structured author name parts for template rendering
    pub author_parts: Vec<AuthorName>,

    /// Title of the work
    pub title: String,

    /// Publication year
    pub year: Option<String>,

    /// Canonical raw BibTeX text for this entry
    pub raw_bibtex: String,

    /// Additional fields (journal, publisher, volume, pages, etc.)
    pub fields: HashMap<String, String>,
}

/// Structured representation of an author name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorName {
    /// Given name(s)
    pub first: String,
    /// Family name
    pub last: String,
    /// Normalized full name
    pub full: String,
}

impl BibTeXEntry {
    /// Create a new BibTeX entry from parsed data
    pub fn new(key: String, entry_type: String, authors: Vec<String>, title: String) -> Self {
        let author_parts = authors
            .iter()
            .map(|a| Self::normalize_author(a))
            .collect::<Vec<_>>();

        Self {
            key,
            entry_type,
            authors,
            author_parts,
            title,
            year: None,
            raw_bibtex: String::new(),
            fields: HashMap::new(),
        }
    }

    /// Set the year field
    pub fn with_year(mut self, year: String) -> Self {
        self.year = Some(year);
        self
    }

    /// Add a field to the entry
    pub fn with_field(mut self, key: String, value: String) -> Self {
        self.fields.insert(key, value);
        self
    }

    /// Set raw BibTeX representation
    pub fn with_raw_bibtex(mut self, raw_bibtex: String) -> Self {
        self.raw_bibtex = raw_bibtex;
        self
    }

    /// Get a field value as a String
    pub fn get_field(&self, field: &str) -> Option<&String> {
        self.fields.get(field)
    }

    fn normalize_author(author: &str) -> AuthorName {
        BibTeXParser::normalize_author_name(author)
    }
}

/// Parser for BibTeX files
pub struct BibTeXParser;

impl BibTeXParser {
    /// Parse a single BibTeX file and return entries
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Vec<BibTeXEntry>> {
        let path = path.as_ref();

        // Read the file content
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        Self::parse_str(&content)
    }

    /// Parse BibTeX entries from a source string
    pub fn parse_str(src: &str) -> Result<Vec<BibTeXEntry>> {
        // Parse using BibLatex
        let bibliography =
            Bibliography::parse(src).map_err(|e| ParseError::Parse(e.to_string()))?;

        if bibliography.is_empty() {
            return Err(ParseError::NoEntries("input".to_string()).into());
        }

        // Convert to our internal representation
        let parsed_entries = bibliography
            .into_iter()
            .map(Self::convert_entry)
            .collect::<Vec<BibTeXEntry>>();

        Ok(parsed_entries)
    }

    /// Parse all BibTeX files in a directory
    pub fn parse_directory<P: AsRef<Path>>(path: P, recursive: bool) -> Result<Vec<BibTeXEntry>> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ParseError::DirectoryError(format!(
                "Directory does not exist: {}",
                path.display()
            ))
            .into());
        }

        if !path.is_dir() {
            return Err(ParseError::DirectoryError(format!(
                "Path is not a directory: {}",
                path.display()
            ))
            .into());
        }

        let mut entries = Vec::new();

        // Collect all .bib files
        let bib_files = if recursive {
            Self::collect_bib_files_recursive(path)?
        } else {
            Self::collect_bib_files_flat(path)?
        };

        // Parse each file
        for file in bib_files {
            match Self::parse_file(&file) {
                Ok(file_entries) => entries.extend(file_entries),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", file.display(), e);
                }
            }
        }

        if entries.is_empty() {
            return Err(ParseError::NoEntries(path.display().to_string()).into());
        }

        Ok(entries)
    }

    /// Collect .bib files from a directory (flat, non-recursive)
    fn collect_bib_files_flat<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
        let path = path.as_ref();
        let mut files = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "bib") {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// Collect .bib files from a directory recursively
    fn collect_bib_files_recursive<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        Self::collect_recursive(path.as_ref(), &mut files)?;
        Ok(files)
    }

    /// Recursive helper to collect .bib files
    fn collect_recursive<P: AsRef<Path>>(path: P, files: &mut Vec<PathBuf>) -> Result<()> {
        let path = path.as_ref();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "bib") {
                files.push(path);
            } else if path.is_dir() {
                Self::collect_recursive(&path, files)?;
            }
        }

        Ok(())
    }

    /// Convert BibLatex entry to our internal representation
    fn convert_entry(entry: biblatex::Entry) -> BibTeXEntry {
        let biblatex::Entry {
            key,
            entry_type,
            fields: all_fields,
        } = entry;
        let entry_type = entry_type.to_string();

        // Extract authors
        let author_parts = all_fields
            .get("author")
            .map(|authors| Self::parse_authors(authors))
            .unwrap_or_default();
        let authors = author_parts
            .iter()
            .map(|a| a.full.clone())
            .collect::<Vec<_>>();

        // Extract title
        let title = all_fields
            .get("title")
            .map(|t| t.format_verbatim())
            .unwrap_or_default();

        // Extract year
        let year = all_fields.get("year").map(|y| y.format_verbatim());

        let raw_bibtex = Self::build_raw_bibtex(&key, &entry_type, &all_fields);

        // Build fields map with remaining fields
        let mut fields = HashMap::new();
        for (k, v) in all_fields {
            if k != "author" && k != "title" && k != "year" {
                let mut value = v.format_verbatim();
                if k == "month" {
                    value = Self::normalize_month_value(&value);
                }

                fields.insert(k, value);
            }
        }

        BibTeXEntry {
            key,
            entry_type,
            authors,
            author_parts,
            title,
            year,
            raw_bibtex,
            fields,
        }
    }

    fn build_raw_bibtex(
        key: &str,
        entry_type: &str,
        fields: &std::collections::BTreeMap<String, biblatex::Chunks>,
    ) -> String {
        let mut raw = format!("@{}{{{},\n", entry_type, key);

        for (field, value) in fields {
            raw.push_str(&format!("  {} = {{{}}},\n", field, value.format_verbatim()));
        }

        raw.push('}');
        raw
    }

    fn normalize_month_value(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return value.to_string();
        }

        if let Ok(month_num) = trimmed.parse::<u8>() {
            if (1..=12).contains(&month_num) {
                return format!("{:02}", month_num);
            }

            return trimmed.to_string();
        }

        let normalized = trimmed.trim_end_matches('.').to_ascii_lowercase();

        match normalized.as_str() {
            "january" | "jan" => "01".to_string(),
            "february" | "feb" => "02".to_string(),
            "march" | "mar" => "03".to_string(),
            "april" | "apr" => "04".to_string(),
            "may" => "05".to_string(),
            "june" | "jun" => "06".to_string(),
            "july" | "jul" => "07".to_string(),
            "august" | "aug" => "08".to_string(),
            "september" | "sep" | "sept" => "09".to_string(),
            "october" | "oct" => "10".to_string(),
            "november" | "nov" => "11".to_string(),
            "december" | "dec" => "12".to_string(),
            _ => trimmed.to_string(),
        }
    }

    /// Parse author field (can be "Last, First" or "First Last" format)
    fn parse_authors(authors: &[biblatex::Spanned<biblatex::Chunk>]) -> Vec<AuthorName> {
        let mut result = Vec::new();
        let authors_text = authors.format_verbatim();

        // Split by "and" to handle multiple authors
        for author in authors_text.split(" and ") {
            let author = author.trim();
            if !author.is_empty() {
                result.push(Self::normalize_author_name(author));
            }
        }

        result
    }

    fn normalize_author_name(author: &str) -> AuthorName {
        let trimmed = author.trim();

        if let Some((last, first)) = trimmed.split_once(',') {
            let first = first.trim().to_string();
            let last = last.trim().to_string();
            let full = if first.is_empty() {
                last.clone()
            } else if last.is_empty() {
                first.clone()
            } else {
                format!("{} {}", first, last)
            };
            return AuthorName { first, last, full };
        }

        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() <= 1 {
            let first = trimmed.to_string();
            return AuthorName {
                first: first.clone(),
                last: String::new(),
                full: first,
            };
        }

        let last = parts.last().unwrap_or(&"").to_string();
        let first = parts[..parts.len() - 1].join(" ");
        let full = format!("{} {}", first, last).trim().to_string();

        AuthorName { first, last, full }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authors_single() {
        let authors = BibTeXParser::parse_authors(&vec![biblatex::Spanned::zero(
            biblatex::Chunk::Normal("John Doe".to_string()),
        )]);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].full, "John Doe");
        assert_eq!(authors[0].first, "John");
        assert_eq!(authors[0].last, "Doe");
    }

    #[test]
    fn test_parse_authors_multiple() {
        let authors = BibTeXParser::parse_authors(&vec![biblatex::Spanned::zero(
            biblatex::Chunk::Normal("John Doe and Jane Smith".to_string()),
        )]);
        assert_eq!(authors.len(), 2);
        assert!(authors[0].full == "John Doe" || authors[0].full == "Jane Smith");
    }

    #[test]
    fn test_parse_authors_last_first() {
        let authors = BibTeXParser::parse_authors(&vec![biblatex::Spanned::zero(
            biblatex::Chunk::Normal("Doe, John".to_string()),
        )]);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].first, "John");
        assert_eq!(authors[0].last, "Doe");
        assert_eq!(authors[0].full, "John Doe");
    }

    #[test]
    fn test_bibtex_entry_creation() {
        let entry = BibTeXEntry::new(
            "test2024".to_string(),
            "article".to_string(),
            vec!["Author One".to_string()],
            "Test Title".to_string(),
        )
        .with_year("2024".to_string())
        .with_raw_bibtex("@article{test2024,\n}\n".to_string())
        .with_field("journal".to_string(), "Test Journal".to_string());

        assert_eq!(entry.key, "test2024");
        assert_eq!(entry.entry_type, "article");
        assert_eq!(entry.authors.len(), 1);
        assert_eq!(entry.year, Some("2024".to_string()));
        assert!(entry.raw_bibtex.contains("@article{test2024"));
        assert_eq!(
            entry.fields.get("journal"),
            Some(&"Test Journal".to_string())
        );
    }

    #[test]
    fn test_parse_non_standard_fields() {
        let src = r#"
@article{k1,
  author = {Doe, John},
  title = {T},
  year = {2024},
  abstract = {A custom abstract field},
  keywords = {privacy,security},
  customflag = {enabled}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with custom fields");
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(
            entry.fields.get("abstract"),
            Some(&"A custom abstract field".to_string())
        );
        assert_eq!(
            entry.fields.get("keywords"),
            Some(&"privacy,security".to_string())
        );
        assert_eq!(entry.fields.get("customflag"), Some(&"enabled".to_string()));
        assert!(entry.raw_bibtex.contains("@article{k1"));
        assert!(
            entry
                .raw_bibtex
                .contains("abstract = {A custom abstract field}")
        );
    }

    #[test]
    fn test_parse_month_textual_values_to_zero_prefixed_numeric() {
        let src = r#"
@article{k1,
  title = {T},
  month = {Feb}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with textual month");
        let entry = &entries[0];
        assert_eq!(entry.fields.get("month"), Some(&"02".to_string()));
    }

    #[test]
    fn test_parse_month_numeric_values_to_zero_prefixed_numeric() {
        let src = r#"
@article{k1,
  title = {T},
  month = {3}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with numeric month");
        let entry = &entries[0];
        assert_eq!(entry.fields.get("month"), Some(&"03".to_string()));
    }
}
