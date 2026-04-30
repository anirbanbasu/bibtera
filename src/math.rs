#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MathSegment {
    pub(crate) is_math: bool,
    pub(crate) text: String,
}

pub(crate) fn split_math_segments(input: &str) -> Vec<MathSegment> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut text_buffer = String::new();
    let mut segments = Vec::new();

    while index < chars.len() {
        if let Some((math_segment, next_index)) = consume_math_segment(&chars, index) {
            if !text_buffer.is_empty() {
                segments.push(MathSegment {
                    is_math: false,
                    text: std::mem::take(&mut text_buffer),
                });
            }

            segments.push(MathSegment {
                is_math: true,
                text: math_segment,
            });
            index = next_index;
            continue;
        }

        text_buffer.push(chars[index]);
        // When an unclosed `$$` is detected, consume both `$` characters as plain
        // text so the second `$` is not misinterpreted as a single-`$` delimiter.
        if chars[index] == '$'
            && !is_escaped(&chars, index)
            && index + 1 < chars.len()
            && chars[index + 1] == '$'
        {
            text_buffer.push(chars[index + 1]);
            index += 2;
        } else {
            index += 1;
        }
    }

    if !text_buffer.is_empty() {
        segments.push(MathSegment {
            is_math: false,
            text: text_buffer,
        });
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
            let segment = chars[start..index + close_len].iter().collect::<String>();
            return Some((segment, index + close_len));
        }

        index += 1;
    }

    None
}

pub(crate) fn is_escaped(chars: &[char], index: usize) -> bool {
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
