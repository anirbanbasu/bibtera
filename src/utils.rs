//! Utility functions for the BibTeX converter.
//!
//! This module provides common utilities used across the application,
//! including file operations, string formatting, and path handling.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha3::digest::{ExtendableOutput, Update, XofReader};
use shake::Shake128;
use uuid::Uuid;

use crate::config::FileNameStrategy;

/// Read a file only if it resolves inside the permitted root directory.
///
/// Both the requested path and the permitted root are canonicalised before
/// the containment check, so a path that escapes the root through `..`
/// components or symbolic links is rejected rather than followed
/// (NON-FUNC-4).
pub fn safe_read<P: AsRef<Path>, R: AsRef<Path>>(path: P, allowed_root: R) -> Result<String> {
    let path = path.as_ref();
    let allowed_root = allowed_root.as_ref();

    let canonical = path
        .canonicalize()
        .context(format!("Failed to resolve path: {}", path.display()))?;
    let root_canonical = allowed_root.canonicalize().context(format!(
        "Failed to resolve permitted root directory: {}",
        allowed_root.display()
    ))?;

    if !canonical.starts_with(&root_canonical) {
        anyhow::bail!(
            "Refusing to read {} because it resolves to {} outside the permitted directory {}",
            path.display(),
            canonical.display(),
            root_canonical.display()
        );
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

/// Reject temp file name components that contain path separators, so that a
/// prefix or suffix can never move the created file out of the temporary
/// directory (NON-FUNC-4). Both `/` and `\` are rejected on every platform
/// for consistent cross-platform behaviour.
fn validate_temp_name_component(label: &str, value: &str) -> Result<()> {
    if value.contains(['/', '\\']) {
        anyhow::bail!(
            "The temp file {} must not contain path separators: {}",
            label,
            value
        );
    }

    Ok(())
}

/// Create a uniquely named temporary file in the system temporary directory.
///
/// The suffix is appended verbatim after the unique name component, so an
/// extension such as `.txt` is preserved in the created file name. Neither
/// the prefix nor the suffix may contain path separators, so the file is
/// always created directly inside the temporary directory. The file is
/// opened with `create_new` (exclusive creation), so an existing file is
/// never truncated; on a name collision a different unique name is tried.
pub fn create_temp_file(prefix: &str, suffix: &str) -> Result<(PathBuf, fs::File)> {
    const MAX_ATTEMPTS: u32 = 1024;

    validate_temp_name_component("prefix", prefix)?;
    validate_temp_name_component("suffix", suffix)?;

    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();

    for attempt in 0..MAX_ATTEMPTS {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let filename = format!("{}_{:x}_{:x}_{:x}{}", prefix, pid, nanos, attempt, suffix);
        let path = temp_dir.join(filename);

        match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .context(format!("Failed to create temp file: {}", path.display()));
            }
        }
    }

    anyhow::bail!(
        "Failed to create a unique temp file in {} after {} attempts",
        temp_dir.display(),
        MAX_ATTEMPTS
    )
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

/// Truncate a string to a maximum number of characters, adding an ellipsis
/// if content was removed.
///
/// Truncation happens on character boundaries, so multi-byte UTF-8 content
/// is safe, and the result never exceeds `max_len` characters. At most
/// `max_len + 1` characters are examined, so the cost is bounded by
/// `max_len` rather than the input length.
pub fn truncate(s: &str, max_len: usize) -> String {
    let exceeds_max = s.char_indices().nth(max_len).is_some();
    if !exceeds_max {
        return s.to_string();
    }

    if max_len <= 3 {
        return ".".repeat(max_len);
    }

    let prefix_end = s
        .char_indices()
        .nth(max_len - 3)
        .map(|(index, _)| index)
        .unwrap_or(s.len());
    format!("{}...", &s[..prefix_end])
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
        FileNameStrategy::Uuid8 => uuid8_from_key(key),
        FileNameStrategy::Slugify => slugify_key(key),
    };

    format!("{}.{}", stem, extension)
}

/// Disambiguate `filename` against the set of filenames already produced
/// earlier in the same run, so that distinct entries whose keys hash or
/// slugify to the same name (FUNC-2.1) never collide within a single run.
///
/// If `filename` is not in `used`, it is returned unchanged. Otherwise a
/// numeric suffix (`-2`, `-3`, ...) is inserted before the extension until a
/// name outside `used` is found.
pub fn disambiguate_filename(filename: &str, used: &std::collections::BTreeSet<String>) -> String {
    if !used.contains(filename) {
        return filename.to_string();
    }

    let (stem, ext) = match filename.rsplit_once('.') {
        Some((stem, ext)) => (stem, Some(ext)),
        None => (filename, None),
    };

    let mut counter: u32 = 2;
    loop {
        let candidate = match ext {
            Some(ext) => format!("{}-{}.{}", stem, counter, ext),
            None => format!("{}-{}", stem, counter),
        };
        if !used.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

/// Build a UUIDv8 (RFC 9562 custom profile) string from 16 bytes sourced from
/// SHAKE-128(key).
///
/// UUIDv8 is used, rather than v7, because the 128 bits are entirely
/// hash-derived rather than containing a Unix-millisecond timestamp as v7
/// requires: labelling this content as v7 would misrepresent it as
/// time-ordered. The hash-based construction is deliberately kept so that the
/// same key always yields the same file name across runs.
fn uuid8_from_key(key: &str) -> String {
    let mut hasher = Shake128::default();
    hasher.update(key.as_bytes());

    let mut reader = hasher.finalize_xof();
    let mut bytes = [0u8; 16];
    reader.read(&mut bytes);

    // Apply UUIDv8 version and RFC4122 variant bits.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
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

    fn unique_test_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "bibtera_utils_{}_{}_{}",
            label,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("create test directory");
        dir
    }

    #[test]
    fn test_safe_read_allows_files_inside_the_permitted_root() {
        let root = unique_test_dir("safe_read_inside");
        let file = root.join("inside.bib");
        fs::write(&file, "content").expect("write test file");

        let read = safe_read(&file, &root).expect("read file inside root");
        assert_eq!(read, "content");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_safe_read_rejects_paths_outside_the_permitted_root() {
        let root = unique_test_dir("safe_read_root");
        let outside_dir = unique_test_dir("safe_read_outside");
        let outside_file = outside_dir.join("outside.bib");
        fs::write(&outside_file, "secret").expect("write outside file");

        let error = safe_read(&outside_file, &root).expect_err("outside path must be rejected");
        assert!(
            format!("{error:#}").contains("outside the permitted directory"),
            "unexpected error: {error:#}"
        );

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside_dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn test_safe_read_rejects_symlinks_escaping_the_permitted_root() {
        let root = unique_test_dir("safe_read_symlink_root");
        let outside_dir = unique_test_dir("safe_read_symlink_target");
        let target = outside_dir.join("target.bib");
        fs::write(&target, "secret").expect("write symlink target");

        let link = root.join("escape.bib");
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");

        let error = safe_read(&link, &root).expect_err("escaping symlink must be rejected");
        assert!(
            format!("{error:#}").contains("outside the permitted directory"),
            "unexpected error: {error:#}"
        );

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside_dir).ok();
    }

    #[test]
    fn test_create_temp_file_preserves_suffix_extension() {
        let (path, _file) = create_temp_file("bibtera_suffix", ".txt").expect("create temp file");
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("temp file name");

        assert!(name.starts_with("bibtera_suffix"), "name: {name}");
        assert!(name.ends_with(".txt"), "name: {name}");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_create_temp_file_preserves_full_suffix_including_extension() {
        let (path, _file) =
            create_temp_file("bibtera_suffix", "_data.txt").expect("create temp file");
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("temp file name");

        assert!(name.ends_with("_data.txt"), "name: {name}");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_create_temp_file_repeated_calls_do_not_collide_or_truncate() {
        use std::io::Write as _;

        let (first_path, mut first_file) =
            create_temp_file("bibtera_collide", ".tmp").expect("create first temp file");
        first_file
            .write_all(b"original")
            .expect("write first temp file");

        let (second_path, _second_file) =
            create_temp_file("bibtera_collide", ".tmp").expect("create second temp file");

        assert_ne!(first_path, second_path);
        assert_eq!(
            fs::read_to_string(&first_path).expect("read first temp file"),
            "original"
        );

        fs::remove_file(&first_path).ok();
        fs::remove_file(&second_path).ok();
    }

    #[test]
    fn test_create_temp_file_rejects_path_separators_in_prefix_and_suffix() {
        for (prefix, suffix) in [
            ("../escape", ".txt"),
            ("/etc/bibtera", ".txt"),
            ("bibtera", "/etc/passwd"),
            ("bibtera", "..\\escape.txt"),
        ] {
            let error = create_temp_file(prefix, suffix)
                .expect_err("separator-bearing components must be rejected");
            assert!(
                format!("{error:#}").contains("must not contain path separators"),
                "unexpected error for ({prefix}, {suffix}): {error:#}"
            );
        }
    }

    #[test]
    fn test_create_temp_file_concurrent_calls_produce_distinct_paths() {
        let handles = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    create_temp_file("bibtera_concurrent", ".tmp")
                        .expect("create temp file concurrently")
                        .0
                })
            })
            .collect::<Vec<_>>();

        let paths = handles
            .into_iter()
            .map(|handle| handle.join().expect("join temp file thread"))
            .collect::<Vec<_>>();
        let unique = paths.iter().collect::<std::collections::BTreeSet<_>>();

        assert_eq!(unique.len(), paths.len());

        for path in &paths {
            fs::remove_file(path).ok();
        }
    }

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
        assert_eq!(truncate("hello!", 6), "hello!");
        assert_eq!(truncate("hello!!", 6), "hel...");
    }

    #[test]
    fn test_truncate_non_ascii_input_is_safe() {
        // Regression: byte-indexed slicing used to panic inside 'é'.
        assert_eq!(truncate("héllo wörld", 5), "hé...");
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("日本語のタイトル", 6), "日本語...");
    }

    #[test]
    fn test_truncate_never_exceeds_requested_maximum() {
        for max_len in 0..8 {
            let result = truncate("héllo wörld", max_len);
            assert!(
                result.chars().count() <= max_len,
                "truncate to {} produced {:?}",
                max_len,
                result
            );
        }
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
    fn test_generate_output_filename_uuid8() {
        let name = generate_output_filename("test-key", FileNameStrategy::Uuid8, "txt");
        assert!(name.ends_with(".txt"));
        assert_eq!(name.matches('-').count(), 4);
    }

    #[test]
    fn test_generate_output_filename_uuid8_has_version_and_variant_bits() {
        let name = generate_output_filename("test-key", FileNameStrategy::Uuid8, "txt");
        let stem = name.strip_suffix(".txt").expect("stem");
        assert_eq!(stem.chars().nth(14), Some('8'), "version nibble: {stem}");
        let variant_nibble = stem.chars().nth(19).expect("variant nibble");
        assert!(
            matches!(variant_nibble, '8' | '9' | 'a' | 'b'),
            "variant nibble: {stem}"
        );
    }

    #[test]
    fn test_disambiguate_filename_returns_unchanged_when_not_seen() {
        let used = std::collections::BTreeSet::new();
        assert_eq!(disambiguate_filename("entry.md", &used), "entry.md");
    }

    #[test]
    fn test_disambiguate_filename_appends_counter_before_extension() {
        let mut used = std::collections::BTreeSet::new();
        used.insert("entry.md".to_string());
        assert_eq!(disambiguate_filename("entry.md", &used), "entry-2.md");
    }

    #[test]
    fn test_disambiguate_filename_skips_already_used_counters() {
        let mut used = std::collections::BTreeSet::new();
        used.insert("entry.md".to_string());
        used.insert("entry-2.md".to_string());
        used.insert("entry-3.md".to_string());
        assert_eq!(disambiguate_filename("entry.md", &used), "entry-4.md");
    }

    #[test]
    fn test_disambiguate_filename_handles_missing_extension() {
        let mut used = std::collections::BTreeSet::new();
        used.insert("entry".to_string());
        assert_eq!(disambiguate_filename("entry", &used), "entry-2");
    }
}
