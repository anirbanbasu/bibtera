//! BibTeX parsing module.
//!
//! This module provides functionality to parse BibTeX files using the BibLatex library.
//! It extracts entry types, keys, authors, titles, and other metadata from BibTeX entries.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use biblatex::{Bibliography, Chunk, Person, RawBibliography, RawChunk, Type};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::math::split_math_segments;

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

/// BibTeX fields exposed as top-level template context keys rather than through the `fields` map.
pub const TOP_LEVEL_FIELD_KEYS: [&str; 3] = ["author", "title", "year"];

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

    /// Keywords slugified for template usage
    pub slugified_keywords: Vec<String>,

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
    /// Normalised full name
    pub full: String,
}

impl BibTeXEntry {
    /// Create a new BibTeX entry from parsed data
    pub fn new(key: String, entry_type: String, authors: Vec<String>, title: String) -> Self {
        let author_parts = authors
            .iter()
            .map(|a| Self::normalise_author(a))
            .collect::<Vec<_>>();

        Self {
            key,
            entry_type,
            authors,
            author_parts,
            title,
            year: None,
            raw_bibtex: String::new(),
            slugified_keywords: Vec::new(),
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

    /// Set slugified keyword list
    pub fn with_slugified_keywords(mut self, slugified_keywords: Vec<String>) -> Self {
        self.slugified_keywords = slugified_keywords;
        self
    }

    /// Get a field value as a String
    pub fn get_field(&self, field: &str) -> Option<&String> {
        self.fields.get(field)
    }

    fn normalise_author(author: &str) -> AuthorName {
        BibTeXParser::normalise_author_name(author)
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
        let raw_bibliography =
            RawBibliography::parse(src).map_err(|e| ParseError::Parse(e.to_string()))?;
        let raw_field_lookup = Self::build_raw_field_lookup(&raw_bibliography);

        // Parse using BibLatex
        let bibliography =
            Bibliography::parse(src).map_err(|e| ParseError::Parse(e.to_string()))?;

        if bibliography.is_empty() {
            return Err(ParseError::NoEntries("input".to_string()).into());
        }

        // Convert to our internal representation
        let parsed_entries = bibliography
            .into_iter()
            .map(|entry| {
                let raw_fields = raw_field_lookup.get(&entry.key);
                Self::convert_entry(entry, raw_fields)
            })
            .collect::<Vec<BibTeXEntry>>();

        Ok(parsed_entries)
    }

    /// Parse all BibTeX files in a directory.
    ///
    /// Every discovered file is read through [`crate::utils::safe_read`] with
    /// the scanned directory as the permitted root, so a `.bib` file that
    /// resolves outside the directory, for example through a symbolic link,
    /// is rejected rather than followed (NON-FUNC-4).
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

        // Parse each file and fail immediately if any file cannot be read
        // within the scanned directory or cannot be parsed.
        for file in bib_files {
            let content = crate::utils::safe_read(&file, path)
                .with_context(|| format!("Failed to read BibTeX file: {}", file.display()))?;
            let file_entries = Self::parse_str(&content)
                .with_context(|| format!("Failed to parse BibTeX file: {}", file.display()))?;
            entries.extend(file_entries);
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
    fn convert_entry(
        entry: biblatex::Entry,
        raw_fields: Option<&HashMap<String, String>>,
    ) -> BibTeXEntry {
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
            .map(|t| {
                Self::format_chunks_preserving_math_syntax(
                    t,
                    raw_fields.and_then(|fields| fields.get("title").map(String::as_str)),
                )
            })
            .unwrap_or_default();

        // Extract year
        let year = all_fields.get("year").map(|y| {
            Self::format_chunks_preserving_math_syntax(
                y,
                raw_fields.and_then(|fields| fields.get("year").map(String::as_str)),
            )
        });

        // Extract keywords as slugified list
        let slugified_keywords = all_fields
            .get("keywords")
            .map(|keywords| {
                Self::parse_slugified_keywords(&Self::format_chunks_preserving_math_syntax(
                    keywords,
                    raw_fields.and_then(|fields| fields.get("keywords").map(String::as_str)),
                ))
            })
            .unwrap_or_default();

        let raw_bibtex = Self::build_raw_bibtex(&key, &entry_type, &all_fields, raw_fields);

        // Build fields map with remaining fields
        let mut fields = HashMap::new();
        for (k, v) in all_fields {
            if !TOP_LEVEL_FIELD_KEYS.contains(&k.as_str()) {
                let mut value = Self::format_chunks_preserving_math_syntax(
                    &v,
                    raw_fields.and_then(|fields| fields.get(&k).map(String::as_str)),
                );
                if k == "month" {
                    value = Self::normalise_month_value(&value);
                } else if k == "day" {
                    value = Self::normalise_day_value(&value);
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
            slugified_keywords,
            fields,
        }
    }

    fn build_raw_bibtex(
        key: &str,
        entry_type: &str,
        fields: &std::collections::BTreeMap<String, biblatex::Chunks>,
        raw_fields: Option<&HashMap<String, String>>,
    ) -> String {
        let mut raw = format!("@{}{{{},\n", entry_type, key);

        for (field, value) in fields {
            raw.push_str(&format!(
                "  {} = {{{}}},\n",
                field,
                Self::format_chunks_preserving_math_syntax(
                    value,
                    raw_fields.and_then(|field_map| field_map.get(field).map(String::as_str)),
                )
            ));
        }

        raw.push('}');
        raw
    }

    fn normalise_month_value(value: &str) -> String {
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

        let normalised = trimmed.trim_end_matches('.').to_ascii_lowercase();

        match normalised.as_str() {
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

    fn normalise_day_value(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return value.to_string();
        }

        if let Ok(day_num) = trimmed.parse::<u8>()
            && (1..=31).contains(&day_num)
        {
            return format!("{:02}", day_num);
        }

        trimmed.to_string()
    }

    fn build_raw_field_lookup(
        raw_bibliography: &RawBibliography<'_>,
    ) -> HashMap<String, HashMap<String, String>> {
        let mut lookup = HashMap::new();

        for entry in &raw_bibliography.entries {
            let mut field_map = HashMap::new();

            for pair in &entry.v.fields {
                let field_name = pair.key.v.to_ascii_lowercase();
                let mut raw_value = String::new();

                for chunk in &pair.value.v {
                    match chunk.v {
                        RawChunk::Normal(value) | RawChunk::Abbreviation(value) => {
                            raw_value.push_str(value)
                        }
                    }
                }

                field_map.insert(field_name, raw_value);
            }

            lookup.insert(entry.v.key.v.to_string(), field_map);
        }

        lookup
    }

    fn format_chunks_preserving_math_syntax(
        chunks: &[biblatex::Spanned<Chunk>],
        original_value: Option<&str>,
    ) -> String {
        let mut output = String::new();
        // Whitespace is buffered so that a run containing a line break, such
        // as the newline and indentation of a line-wrapped BibTeX field value,
        // collapses into a single separator space (FUNC-1.6).
        let mut pending_whitespace = String::new();
        let mut pending_has_line_break = false;

        for chunk in chunks {
            match &chunk.v {
                Chunk::Normal(value) => {
                    for ch in value.chars() {
                        if ch.is_whitespace() {
                            pending_whitespace.push(ch);
                            if ch == '\n' || ch == '\r' {
                                pending_has_line_break = true;
                            }
                        } else {
                            Self::flush_pending_whitespace(
                                &mut output,
                                &mut pending_whitespace,
                                &mut pending_has_line_break,
                            );
                            output.push(ch);
                        }
                    }
                }
                Chunk::Verbatim(value) => {
                    Self::flush_pending_whitespace(
                        &mut output,
                        &mut pending_whitespace,
                        &mut pending_has_line_break,
                    );
                    output.push_str(value);
                }
                Chunk::Math(value) => {
                    Self::flush_pending_whitespace(
                        &mut output,
                        &mut pending_whitespace,
                        &mut pending_has_line_break,
                    );
                    output.push('$');
                    output.push_str(value);
                    output.push('$');
                }
            }
        }

        Self::flush_pending_whitespace(
            &mut output,
            &mut pending_whitespace,
            &mut pending_has_line_break,
        );

        if let Some(original) = original_value {
            return Self::merge_original_math_segments(&output, original);
        }

        output
    }

    fn flush_pending_whitespace(
        output: &mut String,
        pending_whitespace: &mut String,
        pending_has_line_break: &mut bool,
    ) {
        if pending_whitespace.is_empty() {
            return;
        }

        if *pending_has_line_break {
            if !output.ends_with(char::is_whitespace) {
                output.push(' ');
            }
        } else {
            output.push_str(pending_whitespace);
        }

        pending_whitespace.clear();
        *pending_has_line_break = false;
    }

    fn merge_original_math_segments(formatted: &str, original: &str) -> String {
        let formatted_segments = split_math_segments(formatted);
        let original_math_segments = split_math_segments(original)
            .into_iter()
            .filter(|segment| segment.is_math)
            .map(|segment| segment.text)
            .collect::<Vec<_>>();

        if original_math_segments.is_empty() {
            return formatted.to_string();
        }

        // Count math segments to detect parser alignment issues (IF-TPL-1.3)
        let formatted_math_count = formatted_segments.iter().filter(|s| s.is_math).count();

        if formatted_math_count != original_math_segments.len() {
            // Asymmetry detected: more original math segments than formatted ones.
            // This indicates a mismatch between parser and raw-text detection.
            // Fall back to formatted to avoid silently dropping content (IF-TPL-1.3).
            eprintln!(
                "Warning: Math segment count mismatch in merge_original_math_segments \
                 (formatted: {}, original: {}). Using formatted content to preserve all regions.",
                formatted_math_count,
                original_math_segments.len()
            );
            return formatted.to_string();
        }

        let mut output = String::new();
        let mut math_index = 0;

        for formatted_segment in formatted_segments {
            if formatted_segment.is_math {
                if let Some(original_math) = original_math_segments.get(math_index) {
                    output.push_str(original_math);
                } else {
                    output.push_str(&formatted_segment.text);
                }

                math_index += 1;
            } else {
                output.push_str(&formatted_segment.text);
            }
        }

        output
    }

    fn parse_slugified_keywords(value: &str) -> Vec<String> {
        value
            .split([',', ';'])
            .map(str::trim)
            .filter(|keyword| !keyword.is_empty())
            .map(Self::slugify_keyword)
            .filter(|keyword| !keyword.is_empty())
            .collect()
    }

    fn slugify_keyword(keyword: &str) -> String {
        let mut output = String::with_capacity(keyword.len());
        let mut last_was_hyphen = false;

        for ch in keyword.chars() {
            if ch.is_ascii_alphanumeric() {
                output.push(ch.to_ascii_lowercase());
                last_was_hyphen = false;
            } else if !last_was_hyphen {
                output.push('-');
                last_was_hyphen = true;
            }
        }

        output.trim_matches('-').to_string()
    }

    /// Parse the author field into structured name parts, deferring to
    /// biblatex's BibTeX name-list handling so that brace-protected literal
    /// names, whitespace-wrapped separators and von/particle/suffix name
    /// conventions are honoured (FUNC-1.1, FUNC-1.1.1, FUNC-1.1.2).
    fn parse_authors(authors: &[biblatex::Spanned<biblatex::Chunk>]) -> Vec<AuthorName> {
        let normalised = Self::normalise_and_separators(authors);

        Vec::<Person>::from_chunks(&normalised)
            .map(|persons| {
                persons
                    .iter()
                    .map(Self::author_name_from_person)
                    .filter(|author| !author.full.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Lowercase standalone name-separator tokens ("AND", "And") in normal
    /// chunks so that the case-sensitive splitter in biblatex honours
    /// BibTeX's case-insensitive separator rule (FUNC-1.1.1). Only tokens
    /// delimited by whitespace on both sides are rewritten, and
    /// brace-protected (verbatim) content is never touched.
    fn normalise_and_separators(
        chunks: &[biblatex::Spanned<Chunk>],
    ) -> Vec<biblatex::Spanned<Chunk>> {
        chunks
            .iter()
            .map(|chunk| match &chunk.v {
                Chunk::Normal(value) => biblatex::Spanned::new(
                    Chunk::Normal(Self::lowercase_separator_tokens(value)),
                    chunk.span.clone(),
                ),
                _ => chunk.clone(),
            })
            .collect()
    }

    fn lowercase_separator_tokens(value: &str) -> String {
        let mut result = String::with_capacity(value.len());
        let mut token = String::new();
        let mut token_preceded_by_whitespace = false;
        let mut previous_was_whitespace = false;

        for ch in value.chars() {
            if ch.is_whitespace() {
                if token_preceded_by_whitespace && token.eq_ignore_ascii_case("and") {
                    result.push_str("and");
                } else {
                    result.push_str(&token);
                }
                token.clear();
                result.push(ch);
                previous_was_whitespace = true;
            } else {
                if token.is_empty() {
                    token_preceded_by_whitespace = previous_was_whitespace;
                }
                token.push(ch);
                previous_was_whitespace = false;
            }
        }

        // A trailing token has no following whitespace, so it is never a
        // separator and is appended unchanged.
        result.push_str(&token);
        result
    }

    fn normalise_author_name(author: &str) -> AuthorName {
        let chunks = vec![biblatex::Spanned::detached(Chunk::Normal(
            author.to_string(),
        ))];

        Self::author_name_from_person(&Person::parse(&chunks))
    }

    /// Map a biblatex person to the template-facing author parts. The name
    /// prefix (nobiliary particle) belongs to the family name and a suffix
    /// such as "Jr." is appended to the full name after a comma
    /// (FUNC-1.1.2). Internal whitespace is collapsed so that line-wrapped
    /// author fields do not leak indentation into name parts.
    fn author_name_from_person(person: &Person) -> AuthorName {
        let first = Self::collapse_internal_whitespace(&person.given_name);
        let last =
            Self::collapse_internal_whitespace(&format!("{} {}", person.prefix, person.name));
        let suffix = Self::collapse_internal_whitespace(&person.suffix);

        let mut full = Self::collapse_internal_whitespace(&format!("{} {}", first, last));
        if !suffix.is_empty() {
            full = if full.is_empty() {
                suffix
            } else {
                format!("{}, {}", full, suffix)
            };
        }

        AuthorName { first, last, full }
    }

    fn collapse_internal_whitespace(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authors_single() {
        let authors = BibTeXParser::parse_authors(&[biblatex::Spanned::zero(
            biblatex::Chunk::Normal("John Doe".to_string()),
        )]);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].full, "John Doe");
        assert_eq!(authors[0].first, "John");
        assert_eq!(authors[0].last, "Doe");
    }

    #[test]
    fn test_parse_authors_multiple() {
        let authors = BibTeXParser::parse_authors(&[biblatex::Spanned::zero(
            biblatex::Chunk::Normal("John Doe and Jane Smith".to_string()),
        )]);
        assert_eq!(authors.len(), 2);
        assert!(authors[0].full == "John Doe" || authors[0].full == "Jane Smith");
    }

    #[test]
    fn test_parse_authors_last_first() {
        let authors = BibTeXParser::parse_authors(&[biblatex::Spanned::zero(
            biblatex::Chunk::Normal("Doe, John".to_string()),
        )]);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].first, "John");
        assert_eq!(authors[0].last, "Doe");
        assert_eq!(authors[0].full, "John Doe");
    }

    #[test]
    fn test_format_chunks_collapses_line_wrapped_whitespace() {
        let chunks = vec![biblatex::Spanned::zero(biblatex::Chunk::Normal(
            "A Very Long\n         Title Continued".to_string(),
        ))];
        let formatted = BibTeXParser::format_chunks_preserving_math_syntax(&chunks, None);
        assert_eq!(formatted, "A Very Long Title Continued");
    }

    #[test]
    fn test_parse_line_wrapped_field_values_collapse_whitespace() {
        let src = "@article{k1,\n  title = {A Very Long\n           Title Continued},\n  year = {2024},\n  keywords = {alpha beta,\n              gamma delta}\n}\n";

        let entries = BibTeXParser::parse_str(src).expect("parse line-wrapped source");
        let entry = &entries[0];
        assert_eq!(entry.title, "A Very Long Title Continued");
        assert_eq!(
            entry.fields.get("keywords"),
            Some(&"alpha beta, gamma delta".to_string())
        );
        assert_eq!(
            entry.slugified_keywords,
            vec!["alpha-beta".to_string(), "gamma-delta".to_string()]
        );
    }

    #[test]
    fn test_parse_authors_brace_protected_corporate_name() {
        let src = "@book{k1,\n  author = {{Barnes and Noble} and Doe, John},\n  title = {T}\n}\n";

        let entries = BibTeXParser::parse_str(src).expect("parse corporate author source");
        let entry = &entries[0];
        assert_eq!(
            entry.authors,
            vec!["Barnes and Noble".to_string(), "John Doe".to_string()]
        );
        assert_eq!(entry.author_parts[0].last, "Barnes and Noble");
        assert_eq!(entry.author_parts[1].first, "John");
        assert_eq!(entry.author_parts[1].last, "Doe");
    }

    #[test]
    fn test_parse_authors_case_insensitive_and_separator() {
        let src = "@article{k1,\n  author = {Smith, A. AND Jones, B.},\n  title = {T}\n}\n";

        let entries = BibTeXParser::parse_str(src).expect("parse uppercase separator source");
        let entry = &entries[0];
        assert_eq!(entry.authors.len(), 2);
        assert_eq!(entry.author_parts[0].last, "Smith");
        assert_eq!(entry.author_parts[1].last, "Jones");
    }

    #[test]
    fn test_parse_authors_does_not_split_names_containing_and_letters() {
        let src = "@article{k1,\n  author = {Sandy Anderson},\n  title = {T}\n}\n";

        let entries = BibTeXParser::parse_str(src).expect("parse embedded-and source");
        let entry = &entries[0];
        assert_eq!(entry.authors, vec!["Sandy Anderson".to_string()]);
    }

    #[test]
    fn test_parse_authors_line_wrapped_separator() {
        let src =
            "@article{k1,\n  author = {Smith, A. and\n            Jones, B.},\n  title = {T}\n}\n";

        let entries = BibTeXParser::parse_str(src).expect("parse line-wrapped author source");
        let entry = &entries[0];
        assert_eq!(entry.authors.len(), 2);
        assert_eq!(entry.author_parts[0].last, "Smith");
        assert_eq!(entry.author_parts[1].last, "Jones");
        assert_eq!(entry.author_parts[1].first, "B.");
    }

    #[test]
    fn test_normalise_author_name_last_suffix_first() {
        let author = BibTeXParser::normalise_author_name("Doe, Jr., John");
        assert_eq!(author.first, "John");
        assert_eq!(author.last, "Doe");
        assert_eq!(author.full, "John Doe, Jr.");
    }

    #[test]
    fn test_normalise_author_name_particles_in_first_last_form() {
        let author = BibTeXParser::normalise_author_name("Jean de la Fontaine");
        assert_eq!(author.first, "Jean");
        assert_eq!(author.last, "de la Fontaine");
        assert_eq!(author.full, "Jean de la Fontaine");
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
        .with_slugified_keywords(vec!["privacy-security".to_string()])
        .with_field("journal".to_string(), "Test Journal".to_string());

        assert_eq!(entry.key, "test2024");
        assert_eq!(entry.entry_type, "article");
        assert_eq!(entry.authors.len(), 1);
        assert_eq!(entry.year, Some("2024".to_string()));
        assert!(entry.raw_bibtex.contains("@article{test2024"));
        assert_eq!(
            entry.slugified_keywords,
            vec!["privacy-security".to_string()]
        );
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
    keywords = {Privacy & Security,Zero Trust},
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
            Some(&"Privacy & Security,Zero Trust".to_string())
        );
        assert_eq!(entry.fields.get("customflag"), Some(&"enabled".to_string()));
        assert_eq!(
            entry.slugified_keywords,
            vec!["privacy-security".to_string(), "zero-trust".to_string()]
        );
        assert!(entry.raw_bibtex.contains("@article{k1"));
        assert!(
            entry
                .raw_bibtex
                .contains("abstract = {A custom abstract field}")
        );
    }

    #[test]
    fn test_parse_slugified_keywords_from_keywords_field() {
        let src = r#"
@article{k1,
  title = {T},
  keywords = {Privacy & Security, Zero Trust; AI/ML}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with keywords");
        let entry = &entries[0];
        assert_eq!(
            entry.slugified_keywords,
            vec![
                "privacy-security".to_string(),
                "zero-trust".to_string(),
                "ai-ml".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_day_numeric_values_to_zero_prefixed_numeric() {
        let src = r#"
@article{k1,
  title = {T},
  day = {5}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with numeric day");
        let entry = &entries[0];
        assert_eq!(entry.fields.get("day"), Some(&"05".to_string()));
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

    #[test]
    fn test_parse_invalid_bibtex_returns_error() {
        let src = "@article{missing_comma title = {T}}";
        let error = BibTeXParser::parse_str(src).expect_err("invalid BibTeX should fail");
        let error_text = format!("{error:#}");
        assert!(error_text.contains("Failed to parse BibTeX content"));
    }

    #[test]
    fn test_parse_preserves_math_mode_regions_with_latex_commands() {
        let src = r#"
@article{k1,
  title = {outside \textemdash \textasciitilde \textasciicircum; $inline \textemdash \textasciitilde \textasciicircum$; $$display \textemdash \textasciitilde \textasciicircum$$; \(paren \textemdash \textasciitilde \textasciicircum\); \[bracket \textemdash \textasciitilde \textasciicircum\]}
}
"#;

        let entries = BibTeXParser::parse_str(src).expect("parse source with math-mode commands");
        let entry = &entries[0];

        assert!(
            entry
                .title
                .contains("$inline \\textemdash \\textasciitilde \\textasciicircum$")
        );
        assert!(
            entry
                .title
                .contains("$$display \\textemdash \\textasciitilde \\textasciicircum$$")
        );
        assert!(
            entry
                .title
                .contains("\\(paren \\textemdash \\textasciitilde \\textasciicircum\\)")
        );
        assert!(
            entry
                .title
                .contains("\\[bracket \\textemdash \\textasciitilde \\textasciicircum\\]")
        );
    }

    #[test]
    fn test_merge_original_math_segments_unclosed_double_dollar_does_not_misparse_single_dollar() {
        let original = "$$unclosed TOKEN $real$ math TOKEN";
        let formatted = "$$unclosed CHANGED $real$ math CHANGED";

        let merged = BibTeXParser::merge_original_math_segments(formatted, original);

        assert_eq!(merged, formatted);
    }

    #[test]
    fn test_merge_original_math_segments_detects_count_mismatch_and_falls_back_to_formatted() {
        // Simulate a scenario where the formatted content has a different number of math segments
        // than the original content. This could happen if the parser and raw-text detection diverge.
        // Requirement IF-TPL-1.3: the system must preserve all detected math content and issue a
        // diagnostic warning rather than silently dropping content.
        let formatted = "text $math1$ more $math2$ text";
        // Original has only one math region in its segments (one single-$ pair)
        // but formatted has two. The function should detect this mismatch
        // and fall back to formatted content rather than silently dropping the second math segment.
        let original = "text $single_original$ more text";

        let merged = BibTeXParser::merge_original_math_segments(formatted, original);

        // Result should be formatted to preserve all detected math content
        assert_eq!(merged, formatted);
    }
}
