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

// Matches a single token from the pattern against a single character from the input.
// Returns a tuple of (length of the token in the pattern, whether it matches).
fn match_token(input_char: char, pattern: &str) -> Option<(usize, bool)> {
    match pattern.chars().next() {
        None => None,
        Some('\\') => {
            if pattern.len() < 2 {
                return None;
            }
            let escape_char = pattern.chars().nth(1).unwrap();
            let matches = match escape_char {
                'd' => input_char.is_digit(10),
                'w' => input_char.is_alphanumeric() || input_char == '_',
                _ => return None, // Unknown escape sequence
            };
            Some((2, matches))
        }
        // match the ^ to check if the string starts with the pattern
        Some('^') => {
            if pattern.len() < 2 {
                return None;
            }
            let matches = pattern.chars().nth(1).unwrap() == input_char;
            Some((2, matches))
        }
        Some('[') => match_character_group(input_char, pattern),
        Some(first_char) => Some((1, first_char == input_char)),
    }
}

// Recursively tries to match the rest of the pattern from the current position in the input.
fn match_substring(input: &[char], pattern: &str) -> bool {
    // If the pattern is empty, we've successfully matched everything.
    if pattern.is_empty() {
        return true;
    }
    // If the input is empty but the pattern isn't, no match is possible.
    if input.is_empty() {
        return false;
    }

    // Try to match the first token of the pattern with the current character of the input.
    if let Some((token_len, matches)) = match_token(input[0], pattern) {
        if matches {
            // If it matches, try to match the rest of the pattern with the rest of the input.
            return match_substring(&input[1..], &pattern[token_len..]);
        }
    }

    // If the token doesn't match, this substring attempt has failed.
    false
}

// Tries to match the pattern at any position in the input line.
fn match_pattern(input_line: &str, pattern: &str) -> bool {
    let input_chars: Vec<char> = input_line.chars().collect();
    // Iterate through the input string, treating each character as a potential start of a match.
    for i in 0..input_chars.len() {
        if match_substring(&input_chars[i..], pattern) {
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

    if match_pattern(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
