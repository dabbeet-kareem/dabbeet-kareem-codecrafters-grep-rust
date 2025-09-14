use std::env;
use std::io;
use std::process;

// AST for regex
#[derive(Debug, Clone)]
// A small AST representing our regex. We only implement what we support.
// - Seq: concatenation
// - Alt: alternation (a|b)
// - Repeat: postfix quantifiers (?, +)
// - Start/EndAnchor: ^ and $
// - Dot/Digit/Word/CharClass/Literal: primitive atoms
enum RegexNode {
    Seq(Vec<RegexNode>),
    Alt(Vec<RegexNode>),
    Repeat {
        node: Box<RegexNode>,
        kind: RepeatKind,
    },
    StartAnchor,
    EndAnchor,
    Dot,
    Digit,
    Word,
    CharClass {
        chars: Vec<char>,
        negated: bool,
    },
    Literal(char),
}

#[derive(Debug, Clone, Copy)]
// The only quantifiers we currently support
enum RepeatKind {
    ZeroOrOne,
    OneOrMore,
}

// A tiny recursive-descent parser (EBNF):
//   alt := seq ('|' seq)*
//   seq := repeat*
//   repeat := atom ('?' | '+')?
//   atom := '(' alt ')' | '[' '^'? class ']' | '\\' esc | '.' | '^' | '$' | literal
struct Parser<'a> {
    pattern: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(pattern: &'a str) -> Self {
        Self { pattern, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.pattern[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn expect(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse(&mut self) -> RegexNode {
        self.parse_alt()
    }

    fn parse_alt(&mut self) -> RegexNode {
        let mut branches = Vec::new();
        branches.push(self.parse_seq());
        while self.peek() == Some('|') {
            self.advance();
            branches.push(self.parse_seq());
        }
        if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            RegexNode::Alt(branches)
        }
    }

    fn parse_seq(&mut self) -> RegexNode {
        let mut nodes = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ')' || ch == '|' {
                break;
            }
            nodes.push(self.parse_repeat());
        }
        RegexNode::Seq(nodes)
    }

    fn parse_repeat(&mut self) -> RegexNode {
        let atom = self.parse_atom();
        match self.peek() {
            Some('?') => {
                self.advance();
                RegexNode::Repeat {
                    node: Box::new(atom),
                    kind: RepeatKind::ZeroOrOne,
                }
            }
            Some('+') => {
                self.advance();
                RegexNode::Repeat {
                    node: Box::new(atom),
                    kind: RepeatKind::OneOrMore,
                }
            }
            _ => atom,
        }
    }

    fn parse_atom(&mut self) -> RegexNode {
        match self.peek() {
            Some('(') => {
                self.advance();
                let node = self.parse_alt();
                let _ = self.expect(')');
                node
            }
            Some('[') => self.parse_char_class(),
            Some('\\') => {
                self.advance();
                match self.advance() {
                    Some('d') => RegexNode::Digit,
                    Some('w') => RegexNode::Word,
                    Some(c) => RegexNode::Literal(c),
                    None => RegexNode::Literal('\\'),
                }
            }
            Some('.') => {
                self.advance();
                RegexNode::Dot
            }
            Some('^') => {
                self.advance();
                RegexNode::StartAnchor
            }
            Some('$') => {
                self.advance();
                RegexNode::EndAnchor
            }
            Some(c) => {
                self.advance();
                RegexNode::Literal(c)
            }
            None => RegexNode::Seq(vec![]),
        }
    }

    fn parse_char_class(&mut self) -> RegexNode {
        let _ = self.advance(); // consume '['
        let negated = if self.peek() == Some('^') {
            self.advance();
            true
        } else {
            false
        };
        let mut chars_in_class = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ']' {
                break;
            }
            chars_in_class.push(self.advance().unwrap());
        }
        let _ = self.expect(']');
        RegexNode::CharClass {
            chars: chars_in_class,
            negated,
        }
    }
}

// Match a node against input at position `pos`, returning all possible end positions.
// We use Vec<char> for Unicode-safety; no byte slicing.
fn match_node(node: &RegexNode, input: &[char], pos: usize) -> Vec<usize> {
    match node {
        RegexNode::Literal(c) => {
            if pos < input.len() && input[pos] == *c {
                vec![pos + 1]
            } else {
                vec![]
            }
        }
        RegexNode::Dot => {
            if pos < input.len() {
                vec![pos + 1]
            } else {
                vec![]
            }
        }
        RegexNode::Digit => {
            if pos < input.len() && input[pos].is_digit(10) {
                vec![pos + 1]
            } else {
                vec![]
            }
        }
        RegexNode::Word => {
            if pos < input.len() && (input[pos].is_alphanumeric() || input[pos] == '_') {
                vec![pos + 1]
            } else {
                vec![]
            }
        }
        RegexNode::CharClass { chars, negated } => {
            if pos >= input.len() {
                return vec![];
            }
            let contains = chars.contains(&input[pos]);
            if (*negated && !contains) || (!*negated && contains) {
                vec![pos + 1]
            } else {
                vec![]
            }
        }
        RegexNode::StartAnchor => {
            if pos == 0 {
                vec![pos]
            } else {
                vec![]
            }
        }
        RegexNode::EndAnchor => {
            if pos == input.len() {
                vec![pos]
            } else {
                vec![]
            }
        }
        RegexNode::Seq(nodes) => {
            // Accumulate possible positions as we progress through the sequence
            let mut positions = vec![pos];
            for n in nodes {
                let mut next_positions = Vec::new();
                for p in positions {
                    let res = match_node(n, input, p);
                    next_positions.extend(res);
                }
                if next_positions.is_empty() {
                    return vec![];
                }
                next_positions.sort_unstable();
                next_positions.dedup();
                positions = next_positions;
            }
            positions
        }
        RegexNode::Alt(branches) => {
            let mut all_positions = Vec::new();
            for br in branches {
                let res = match_node(br, input, pos);
                all_positions.extend(res);
            }
            all_positions.sort_unstable();
            all_positions.dedup();
            all_positions
        }
        RegexNode::Repeat { node: inner, kind } => match kind {
            RepeatKind::ZeroOrOne => {
                // Either skip it or take one
                let mut positions = vec![pos];
                positions.extend(match_node(inner, input, pos));
                positions.sort_unstable();
                positions.dedup();
                positions
            }
            RepeatKind::OneOrMore => {
                // Keep applying `inner` as long as we can, collecting all positions
                let mut results = Vec::new();
                let mut frontier = match_node(inner, input, pos);
                while !frontier.is_empty() {
                    for p in &frontier {
                        if !results.contains(p) {
                            results.push(*p);
                        }
                    }
                    // Advance one more repetition from each frontier point
                    let mut next = Vec::new();
                    for p in &frontier {
                        let step = match_node(inner, input, *p);
                        next.extend(step);
                    }
                    next.sort_unstable();
                    next.dedup();
                    frontier = next;
                }
                results.sort_unstable();
                results.dedup();
                results
            }
        },
    }
}

// Try to match at any position (unless ^/$ constrain it via the AST itself)
fn match_pattern(input_line: &str, pattern: &str) -> bool {
    let mut parser = Parser::new(pattern);
    let ast = parser.parse();
    let input_chars: Vec<char> = input_line.chars().collect();
    for start in 0..=input_chars.len() {
        if !match_node(&ast, &input_chars, start).is_empty() {
            return true;
        }
    }
    false
}

// Recursively tries to match the pattern from the current position in the input.
#[allow(dead_code)]
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
                // Greedily match more of the same token first.
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
