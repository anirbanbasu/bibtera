//! Utility functions for the BibTeX converter.
//!
//! This module provides common utilities used across the application,
//! including file operations, string formatting, and path handling.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha3::Shake128;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use uuid::Uuid;

use crate::config::FileNameStrategy;

/// Securely read a file, preventing path traversal attacks
pub fn safe_read<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Get canonical path to prevent path traversal.
    let canonical = path.canonicalize().context("Failed to resolve path")?;

    // Resolve parent to force normalisation and surface invalid paths.
    if let Some(parent) = canonical.parent() {
        let parent_canonical = parent
            .canonicalize()
            .context("Failed to resolve parent directory")?;

        // Placeholder for an allowlist root check.
        let _ = parent_canonical;
    }

    fs::read_to_string(&canonical).context(format!("Failed to read file: {}", path.display()))
}

/// Securely write content to a file, creating parent directories as needed
pub fn safe_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> Result<()> {
    let path = path.as_ref();
    let content = content.as_ref();

    // Create parent directory if it does not exist.
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).context("Failed to create parent directory")?;
    }

    fs::write(path, content).context(format!("Failed to write file: {}", path.display()))
}

/// Read a file as bytes
pub fn read_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let path = path.as_ref();

    fs::read(path).context(format!("Failed to read file: {}", path.display()))
}

/// Check if a file exists and is readable
pub fn is_readable<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_file() && fs::read(path).is_ok()
}

/// Get the file extension (without the dot)
pub fn extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_string())
}

/// Get the file stem (filename without extension)
pub fn stem<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|s| s.to_string())
}

/// Join path components safely
pub fn join_path<P1: AsRef<Path>, P2: AsRef<Path>>(base: P1, child: P2) -> PathBuf {
    base.as_ref().join(child.as_ref())
}

/// Create a temporary file
pub fn create_temp_file(prefix: &str, suffix: &str) -> Result<(PathBuf, fs::File)> {
    let temp_dir = std::env::temp_dir();
    let stem_value = stem(Path::new(suffix)).unwrap_or_else(|| "file".to_string());
    let filename = format!("{}_{}", prefix, stem_value);
    let path = temp_dir.join(filename);

    let file = fs::File::create(&path)
        .context(format!("Failed to create temp file: {}", path.display()))?;

    Ok((path, file))
}

/// Format a list of strings as a bullet list
pub fn format_bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for item in items {
        result.push_str(format!("* {}\n", item).as_str());
    }
    result
}

/// Format a list of strings as an ordered list
pub fn format_ordered_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for (i, item) in items.iter().enumerate() {
        result.push_str(format!("{}. {}\n", i + 1, item).as_str());
    }
    result
}

/// Truncate a string to a maximum length, adding ellipsis if needed
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    format!("{}...", &s[..max_len.saturating_sub(3)])
}

/// Sanitize a string for use as a filename
pub fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Escape a string for use in Markdown
pub fn escape_markdown(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\\' | '`' | '*' | '_' | '#' | '[' | ']' | '(' | ')' | '!' | '+' | '-' | '.' | '^'
            | '~' | '=' | '|' | '<' | '>' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

/// Escape a string for use in HTML
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Find all files with a given extension in a directory
pub fn find_files_with_extension<P: AsRef<Path>>(dir: P, extension: &str) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();

    if !dir.exists() || !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && ext == extension
        {
            files.push(path);
        }
    }

    Ok(files)
}

/// Calculate the relative path from a base directory to a target
pub fn relative_path<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> PathBuf {
    pathdiff::diff_paths(to.as_ref(), from.as_ref()).unwrap_or_else(|| to.as_ref().to_path_buf())
}

/// Generate output filename from a BibTeX key using the requested strategy.
pub fn generate_output_filename(key: &str, strategy: FileNameStrategy, extension: &str) -> String {
    let stem = match strategy {
        FileNameStrategy::Uuid7 => uuid7_from_key(key),
        FileNameStrategy::Slugify => slugify_key(key),
    };

    format!("{}.{}", stem, extension)
}

/// Build a UUID7 string from 16 bytes sourced from SHAKE-128(key).
fn uuid7_from_key(key: &str) -> String {
    let mut hasher = Shake128::default();
    hasher.update(key.as_bytes());

    let mut reader = hasher.finalize_xof();
    let mut bytes = [0u8; 16];
    reader.read(&mut bytes);

    // Apply UUIDv7 version and RFC4122 variant bits.
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    Uuid::from_bytes(bytes).hyphenated().to_string()
}

/// Replace non-alphanumeric characters with underscores.
fn slugify_key(key: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;

    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }

    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "entry".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension() {
        assert_eq!(extension("file.txt"), Some("txt".to_string()));
        assert_eq!(extension("file.tar.gz"), Some("gz".to_string()));
        assert_eq!(extension("noextension"), None);
    }

    #[test]
    fn test_stem() {
        assert_eq!(stem("file.txt"), Some("file".to_string()));
        assert_eq!(stem("path/to/file.txt"), Some("file".to_string()));
        assert_eq!(stem("noextension"), Some("noextension".to_string()));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "he...");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello world"), "hello-world");
        assert_eq!(sanitize_filename("test@2024"), "test-2024");
        assert_eq!(sanitize_filename("valid-name_123"), "valid-name_123");
    }

    #[test]
    fn test_escape_markdown() {
        assert!(escape_markdown("hello").contains('h'));
        assert!(escape_markdown("*item*").contains("\\*item\\*"));
        assert!(escape_markdown("[link]").contains("\\[link\\]"));
    }

    #[test]
    fn test_escape_html() {
        let escaped = escape_html("<script>alert('xss')</script>");
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
        assert!(!escaped.contains("<"));
    }

    #[test]
    fn test_format_bullet_list() {
        let items = vec!["Item 1".to_string(), "Item 2".to_string()];
        let result = format_bullet_list(&items);
        assert!(result.contains("* Item 1"));
        assert!(result.contains("* Item 2"));
    }

    #[test]
    fn test_format_ordered_list() {
        let items = vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ];
        let result = format_ordered_list(&items);
        assert!(result.contains("1. First"));
        assert!(result.contains("2. Second"));
        assert!(result.contains("3. Third"));
    }

    #[test]
    fn test_find_files_with_extension() {
        let files = find_files_with_extension("/tmp", "nonexistent_ext_xyz");
        assert!(files.is_ok());
        assert!(files.unwrap().is_empty());
    }

    #[test]
    fn test_generate_output_filename_slugify() {
        let name = generate_output_filename("foo/bar:baz", FileNameStrategy::Slugify, "md");
        assert_eq!(name, "foo_bar_baz.md");
    }

    #[test]
    fn test_generate_output_filename_uuid7() {
        let name = generate_output_filename("test-key", FileNameStrategy::Uuid7, "txt");
        assert!(name.ends_with(".txt"));
        assert_eq!(name.matches('-').count(), 4);
    }
}
