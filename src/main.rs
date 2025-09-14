use std::env;
use std::io;
use std::process;

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

// Parses the next token from the pattern and returns its length.
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
        _ => Some(1), // Regular character
    }
}

// Matches a single token from the pattern against a single character from the input.
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
            // Re-using match_character_group, but it returns more than we need.
            // A simpler function could be used, but this works.
            match_character_group(input_char, pattern_token)
                .map(|(_, matched)| matched)
                .unwrap_or(false)
        }
        Some(first_char) => first_char == input_char,
    }
}

// Recursively tries to match the rest of the pattern from the current position in the input.
fn match_substring(input: &str, pattern: &str) -> bool {
    // If the pattern is empty, we've successfully matched everything.
    if pattern.is_empty() {
        return true;
    }

    // Handle end-of-string anchor
    if pattern == "$" {
        return input.is_empty();
    }

    let Some(token_len) = parse_token_len(pattern) else {
        return false; // Invalid pattern
    };

    let token = &pattern[..token_len];
    let rest_of_pattern = &pattern[token_len..];

    // Handle quantifiers
    if let Some(quantifier) = rest_of_pattern.chars().next() {
        if quantifier == '?' {
            let pattern_after_quantifier = &rest_of_pattern[1..];
            // 'zero' case: try to match the rest of the pattern without this token
            if match_substring(input, pattern_after_quantifier) {
                return true;
            }
            // 'one' case: match token once, then the rest
            if !input.is_empty() && token_matches(input.chars().next().unwrap(), token) {
                return match_substring(&input[1..], pattern_after_quantifier);
            }
            return false;
        }

        if quantifier == '+' {
            let pattern_after_quantifier = &rest_of_pattern[1..];
            // Must match at least once
            if !input.is_empty() && token_matches(input.chars().next().unwrap(), token) {
                let remaining_input = &input[1..];
                // Greedily match more, or continue with the rest of the pattern
                return match_substring(remaining_input, pattern)
                    || match_substring(remaining_input, pattern_after_quantifier);
            }
            return false;
        }
    }

    // No quantifier, or not a quantifier we handle
    if !input.is_empty() && token_matches(input.chars().next().unwrap(), token) {
        return match_substring(&input[1..], rest_of_pattern);
    }

    false
}

// Tries to match the pattern at any position in the input line.
fn match_pattern(input_line: &str, pattern: &str) -> bool {
    let mut pattern = pattern;

    if pattern.starts_with('^') {
        pattern = &pattern[1..];
        return match_substring(input_line, pattern);
    }

    for i in 0..=input_line.len() {
        if match_substring(&input_line[i..], pattern) {
            return true;
        }
    }
    false
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
