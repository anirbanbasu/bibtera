//! LaTeX substitution utilities.
//!
//! This module provides loading and merging of substitution maps along with
//! helper functions for converting selected LaTeX snippets to plain Unicode text.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// Deterministic substitution map type used by the application.
pub type SubstitutionMap = BTreeMap<String, String>;

const DEFAULT_SUBSTITUTION_MAP_JSON: &str =
    include_str!("../examples/substitution_map_default.json");

const FORMATTING_COMMANDS: &[&str] = &[
    "emph",
    "textit",
    "textbf",
    "texttt",
    "textsc",
    "underline",
    "textrm",
    "textsf",
    "textnormal",
    "mbox",
    "url",
    "nolinkurl",
];

/// Load the built-in default substitution map shipped with the binary.
pub fn load_default_substitution_map() -> Result<SubstitutionMap> {
    serde_json::from_str(DEFAULT_SUBSTITUTION_MAP_JSON)
        .context("Failed to parse built-in default LaTeX substitution map")
}

/// Load a custom substitution map from a JSON file.
pub fn load_substitution_map_file(path: &Path) -> Result<SubstitutionMap> {
    let content = std::fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read LaTeX substitution map file '{}'",
            path.display()
        )
    })?;

    serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse LaTeX substitution map JSON from '{}'",
            path.display()
        )
    })
}

/// Build the effective substitution map by applying optional custom overrides.
pub fn build_substitution_map(
    custom_substitutions: Option<SubstitutionMap>,
) -> Result<SubstitutionMap> {
    let mut substitutions = load_default_substitution_map()?;

    if let Some(custom) = custom_substitutions {
        for (key, value) in custom {
            substitutions.insert(key, value);
        }
    }

    Ok(substitutions)
}

/// Prepare a longest-first ordered substitution list for deterministic replacement.
pub fn ordered_substitutions(substitutions: &SubstitutionMap) -> Vec<(String, String)> {
    let mut ordered = substitutions
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();

    ordered.sort_by(|(left_key, _), (right_key, _)| {
        right_key
            .len()
            .cmp(&left_key.len())
            .then_with(|| left_key.cmp(right_key))
    });

    ordered
}

/// Convert LaTeX snippets in an input string to plain text using ordered substitutions.
pub fn substitute_latex_to_text_with_ordered(
    input: &str,
    ordered_substitutions: &[(String, String)],
) -> String {
    let segments = split_math_segments(input);
    let mut output = String::new();

    for segment in segments {
        match segment {
            Segment::Text(text) => {
                let unwrapped = unwrap_formatting_commands(&text);
                output.push_str(&apply_substitutions(&unwrapped, ordered_substitutions));
            }
            Segment::Math(math) => output.push_str(&math),
        }
    }

    output
}

/// Convert LaTeX snippets in an input string to plain text using a map.
pub fn substitute_latex_to_text(input: &str, substitutions: &SubstitutionMap) -> String {
    let ordered = ordered_substitutions(substitutions);
    substitute_latex_to_text_with_ordered(input, &ordered)
}

fn apply_substitutions(input: &str, ordered_substitutions: &[(String, String)]) -> String {
    let mut output = input.to_string();

    for (from, to) in ordered_substitutions {
        output = output.replace(from, to);
    }

    output
}

#[derive(Debug)]
enum Segment {
    Text(String),
    Math(String),
}

fn split_math_segments(input: &str) -> Vec<Segment> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut text_buffer = String::new();
    let mut segments = Vec::new();

    while index < chars.len() {
        if let Some((math_segment, next_index)) = consume_math_segment(&chars, index) {
            if !text_buffer.is_empty() {
                segments.push(Segment::Text(std::mem::take(&mut text_buffer)));
            }
            segments.push(Segment::Math(math_segment));
            index = next_index;
            continue;
        }

        text_buffer.push(chars[index]);
        index += 1;
    }

    if !text_buffer.is_empty() {
        segments.push(Segment::Text(text_buffer));
    }

    segments
}

fn consume_math_segment(chars: &[char], start: usize) -> Option<(String, usize)> {
    if start >= chars.len() {
        return None;
    }

    if chars[start] == '$' && !is_escaped(chars, start) {
        if start + 1 < chars.len() && chars[start + 1] == '$' {
            return extract_delimited_segment(chars, start, "$$", 2);
        }

        return extract_delimited_segment(chars, start, "$", 1);
    }

    if chars[start] == '\\' && !is_escaped(chars, start) && start + 1 < chars.len() {
        return match chars[start + 1] {
            '(' => extract_delimited_segment(chars, start, "\\)", 2),
            '[' => extract_delimited_segment(chars, start, "\\]", 2),
            _ => None,
        };
    }

    None
}

fn extract_delimited_segment(
    chars: &[char],
    start: usize,
    close: &str,
    open_len: usize,
) -> Option<(String, usize)> {
    let close_chars = close.chars().collect::<Vec<_>>();
    let close_len = close_chars.len();
    let mut index = start + open_len;

    while index + close_len <= chars.len() {
        let is_match = chars[index..index + close_len] == close_chars[..];
        if is_match && !is_escaped(chars, index) {
            let content = chars[start..index + close_len].iter().collect::<String>();
            return Some((content, index + close_len));
        }

        index += 1;
    }
    None
}

fn unwrap_formatting_commands(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut output = String::new();

    while index < chars.len() {
        if chars[index] != '\\' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        if index + 1 >= chars.len() {
            output.push('\\');
            index += 1;
            continue;
        }

        if !chars[index + 1].is_ascii_alphabetic() {
            output.push('\\');
            output.push(chars[index + 1]);
            index += 2;
            continue;
        }

        let command_start = index + 1;
        let mut command_end = command_start;
        while command_end < chars.len() && chars[command_end].is_ascii_alphabetic() {
            command_end += 1;
        }

        let command = chars[command_start..command_end].iter().collect::<String>();

        if !FORMATTING_COMMANDS.contains(&command.as_str()) {
            output.push('\\');
            output.push_str(&command);
            index = command_end;
            continue;
        }

        let mut content_start = command_end;
        while content_start < chars.len() && chars[content_start].is_whitespace() {
            content_start += 1;
        }

        if content_start >= chars.len() || chars[content_start] != '{' {
            output.push('\\');
            output.push_str(&command);
            index = command_end;
            continue;
        }

        if let Some((content, next_index)) = extract_braced_content(&chars, content_start) {
            output.push_str(&unwrap_formatting_commands(&content));
            index = next_index;
            continue;
        }

        output.push('\\');
        output.push_str(&command);
        index = command_end;
    }

    output
}

fn extract_braced_content(chars: &[char], open_brace_index: usize) -> Option<(String, usize)> {
    if chars.get(open_brace_index) != Some(&'{') {
        return None;
    }

    let mut depth: usize = 0;
    let mut index = open_brace_index;
    let mut content = String::new();

    while index < chars.len() {
        let ch = chars[index];
        let escaped = is_escaped(chars, index);

        if ch == '{' && !escaped {
            depth += 1;
            if depth > 1 {
                content.push(ch);
            }
            index += 1;
            continue;
        }

        if ch == '}' && !escaped {
            if depth == 0 {
                return None;
            }

            depth -= 1;
            if depth == 0 {
                return Some((content, index + 1));
            }

            content.push(ch);
            index += 1;
            continue;
        }

        content.push(ch);
        index += 1;
    }

    None
}

fn is_escaped(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let mut slash_count = 0;
    let mut lookback = index;
    while lookback > 0 {
        lookback -= 1;
        if chars[lookback] == '\\' {
            slash_count += 1;
        } else {
            break;
        }
    }

    slash_count % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_latex_to_text_basic_replacements() {
        let mut substitutions = SubstitutionMap::new();
        substitutions.insert("\\\"{o}".to_string(), "ö".to_string());
        substitutions.insert("\\ss".to_string(), "ß".to_string());

        let output = substitute_latex_to_text("G\\\"{o}del and wei\\ss", &substitutions);
        assert_eq!(output, "Gödel and weiß");
    }

    #[test]
    fn test_substitute_latex_to_text_unwraps_nested_formatting_commands() {
        let mut substitutions = SubstitutionMap::new();
        substitutions.insert("\\\"{o}".to_string(), "ö".to_string());
        substitutions.insert("\\'{e}".to_string(), "é".to_string());

        let output = substitute_latex_to_text(
            "\\textbf{b\\\"{o}ld \\emph{and itali\\'{e}c} text}",
            &substitutions,
        );

        assert_eq!(output, "böld and italiéc text");
    }

    #[test]
    fn test_substitute_latex_to_text_keeps_unknown_commands() {
        let substitutions = SubstitutionMap::new();
        let output = substitute_latex_to_text("\\unknown{value}", &substitutions);
        assert_eq!(output, "\\unknown{value}");
    }

    #[test]
    fn test_substitute_latex_to_text_skips_substitutions_inside_all_math_mode_regions() {
        let mut substitutions = SubstitutionMap::new();
        substitutions.insert("\\\"{o}".to_string(), "ö".to_string());

        let input = r#"outside \"{o}; $inline \"{o}$; $$display \"{o}$$; \(paren \"{o}\); \[bracket \"{o}\]"#;

        let output = substitute_latex_to_text(input, &substitutions);

        assert_eq!(
            output,
            r#"outside ö; $inline \"{o}$; $$display \"{o}$$; \(paren \"{o}\); \[bracket \"{o}\]"#
        );
    }

    #[test]
    fn test_substitute_latex_to_text_treats_unclosed_math_markers_as_plain_text() {
        let mut substitutions = SubstitutionMap::new();
        substitutions.insert("\\\"{o}".to_string(), "ö".to_string());

        let output = substitute_latex_to_text(r#"prefix $ unclosed \"{o}"#, &substitutions);
        assert_eq!(output, "prefix $ unclosed ö");
    }

    #[test]
    fn test_substitute_latex_to_text_preserves_unbraced_formatting_commands() {
        let substitutions = SubstitutionMap::new();

        let output = substitute_latex_to_text(r#"\\textbf foo and \\emph    bar"#, &substitutions);
        assert_eq!(output, r#"\\textbf foo and \\emph    bar"#);
    }
}
