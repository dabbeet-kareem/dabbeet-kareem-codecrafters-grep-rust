use std::env;
use std::io;
use std::process;

// Tries to match the pattern at any position in the input line.
fn match_pattern(input_line: &str, pattern: &str) -> bool {
    // Handle start-of-string anchor
    if let Some(stripped_pattern) = pattern.strip_prefix('^') {
        return match_substring(input_line, stripped_pattern);
    }

    // If not anchored, try to match from the beginning of the string.
    if match_substring(input_line, pattern) {
        return true;
    }

    // If the input is empty, there's nowhere else to start a match.
    if input_line.is_empty() {
        return false;
    }

    // If it didn't match from the start, advance one character and try again.
    let first_char_len = input_line.chars().next().unwrap().len_utf8();
    match_pattern(&input_line[first_char_len..], pattern)
}

// Recursively tries to match the pattern from the current position in the input.
fn match_substring(input: &str, pattern: &str) -> bool {
    // Base case 1: Pattern is exhausted, so we have a match.
    if pattern.is_empty() {
        return true;
    }

    // Base case 2: End-of-string anchor. Match only if input is also empty.
    if pattern == "$" {
        return input.is_empty();
    }

    let Some(token_len) = parse_token_len(pattern) else {
        return false;
    };
    let token = &pattern[..token_len];
    let rest_of_pattern = &pattern[token_len..];

    // Handle '?' quantifier (zero or one)
    if let Some(pattern_after_quantifier) = rest_of_pattern.strip_prefix('?') {
        // Greedy: Try matching the token first (one occurrence).
        if let Some(c) = input.chars().next() {
            if token_matches(c, token) {
                let remaining_input = &input[c.len_utf8()..];
                if match_substring(remaining_input, pattern_after_quantifier) {
                    return true;
                }
            }
        }
        // If the 'one' case didn't lead to a match, try the 'zero' case.
        return match_substring(input, pattern_after_quantifier);
    }

    // Handle '+' quantifier (one or more)
    if let Some(pattern_after_quantifier) = rest_of_pattern.strip_prefix('+') {
        if let Some(c) = input.chars().next() {
            if token_matches(c, token) {
                let remaining_input = &input[c.len_utf8()..];
                // Greedy: Try to match more of the same token first.
                // `pattern` still contains the `+`, e.g. `a+b`.
                if match_substring(remaining_input, pattern) {
                    return true;
                }
                // If that fails, try to match the rest of the pattern.
                if match_substring(remaining_input, pattern_after_quantifier) {
                    return true;
                }
            }
        }
        return false; // '+' must match at least once.
    }

    // No quantifier: Match one character and advance.
    if let Some(c) = input.chars().next() {
        if token_matches(c, token) {
            return match_substring(&input[c.len_utf8()..], rest_of_pattern);
        }
    }

    false
}

// Handles matching for positive and negative character groups (e.g., [abc] and [^abc]).
fn match_character_group(input_char: char, pattern: &str) -> Option<(usize, bool)> {
    // Check if it's a negative character group (like [^abc])
    let (is_negative, group_start_index) = if pattern.starts_with("[^") {
        (true, 2)
    } else {
        (false, 1)
    };

    // Find the closing bracket to identify the characters in the group
    if let Some(end_bracket_pos) = pattern[group_start_index..].find(']') {
        let group_chars = &pattern[group_start_index..group_start_index + end_bracket_pos];
        let token_len = group_start_index + end_bracket_pos + 1;
        let is_in_group = group_chars.contains(input_char);

        // Return the match result, inverting it for negative groups
        if is_negative {
            Some((token_len, !is_in_group))
        } else {
            Some((token_len, is_in_group))
        }
    } else {
        None // Unmatched opening bracket
    }
}

// Parses the next token from the pattern and returns its length in bytes.
fn parse_token_len(pattern: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }

    match pattern.chars().next().unwrap() {
        '\\' => {
            if pattern.len() >= 2 {
                Some(2)
            } else {
                None // Incomplete escape sequence
            }
        }
        '[' => {
            let (_is_negative, group_start_index) = if pattern.starts_with("[^") {
                (true, 2)
            } else {
                (false, 1)
            };
            if let Some(end_bracket_pos) = pattern[group_start_index..].find(']') {
                Some(group_start_index + end_bracket_pos + 1)
            } else {
                None // Unmatched opening bracket
            }
        }
        _ => pattern.chars().next().unwrap().len_utf8().into(), // Regular character
    }
}

// Checks if a single character from the input matches a single token from the pattern.
fn token_matches(input_char: char, pattern_token: &str) -> bool {
    match pattern_token.chars().next() {
        None => false,
        Some('\\') => {
            if pattern_token.len() < 2 {
                return false;
            }
            let escape_char = pattern_token.chars().nth(1).unwrap();
            match escape_char {
                'd' => input_char.is_digit(10),
                'w' => input_char.is_alphanumeric() || input_char == '_',
                _ => false, // Unknown escape sequence
            }
        }
        Some('[') => {
            // Re-using match_character_group, but we only need the boolean result.
            match_character_group(input_char, pattern_token)
                .map(|(_, matched)| matched)
                .unwrap_or(false)
        }
        Some('.') => true, // '.' matches any single character
        Some(first_char) => first_char == input_char,
    }
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    // Trim trailing newline for correct '$' anchor matching
    let trimmed_input = input_line.trim_end_matches('\n');

    if match_pattern(trimmed_input, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
