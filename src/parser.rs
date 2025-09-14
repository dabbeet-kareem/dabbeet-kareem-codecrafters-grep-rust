use crate::ast::{RegexNode, RepeatKind};

/// Parser for regular expressions.
///
/// The `Parser` struct holds the pattern and the current position.
/// It also manages group IDs for capturing groups.
pub struct Parser<'a> {
    pub pattern: &'a str,
    pub pos: usize,
    next_group_id: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given pattern.
    pub fn new(pattern: &'a str) -> Self {
        Self {
            pattern,
            pos: 0,
            next_group_id: 1,
        }
    }

    /// Allocate a new group ID for capturing groups.
    fn alloc_group_id(&mut self) -> usize {
        let id = self.next_group_id;
        self.next_group_id += 1;
        id
    }

    /// Peek at the next character in the pattern without advancing.
    fn peek(&self) -> Option<char> {
        self.pattern[self.pos..].chars().next()
    }

    /// Advance the parser by one character and return it.
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Expect a specific character and advance if it matches.
    fn expect(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Entry point for parsing a regex pattern.
    ///
    /// Calls `parse_alt` to parse the full pattern.
    ///
    /// Example:
    /// - Pattern: `a|b` → Alt([Literal('a'), Literal('b')])
    pub fn parse(&mut self) -> RegexNode {
        self.parse_alt()
    }

    /// Parse alternation (`|`) in the pattern.
    ///
    /// Example:
    /// - Pattern: `a|b|c` → Alt([Seq([Literal('a')]), Seq([Literal('b')]), Seq([Literal('c')])])
    /// - Pattern: `abc`   → Seq([Literal('a'), Literal('b'), Literal('c')])
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

    /// Parse a sequence of regex atoms (concatenation).
    ///
    /// Example:
    /// - Pattern: `abc` → Seq([Literal('a'), Literal('b'), Literal('c')])
    /// - Pattern: `a(b|c)d` → Seq([Literal('a'), Group, Literal('d')])
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

    /// Parse repetition operators (`?`, `+`) after an atom.
    ///
    /// Example:
    /// - Pattern: `a?` → Repeat { node: Literal('a'), kind: ZeroOrOne }
    /// - Pattern: `b+` → Repeat { node: Literal('b'), kind: OneOrMore }
    /// - Pattern: `c`  → Literal('c')
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

    /// Parse a single regex atom: group, char class, escape, literal, or anchor.
    ///
    /// Examples:
    /// - Pattern: `(abc)` → Group { id, node: Seq([Literal('a'), Literal('b'), Literal('c')]) }
    /// - Pattern: `[abc]` → CharClass { chars: ['a','b','c'], negated: false }
    /// - Pattern: `\d`   → Digit
    /// - Pattern: `\w`   → Word
    /// - Pattern: `\1`   → BackRef { id: 1 }
    /// - Pattern: `.`    → Dot
    /// - Pattern: `^`    → StartAnchor
    /// - Pattern: `$`    → EndAnchor
    /// - Pattern: `a`    → Literal('a')
    fn parse_atom(&mut self) -> RegexNode {
        match self.peek() {
            Some('(') => {
                // Parse a capturing group: ( ... )
                self.advance();
                let id = self.alloc_group_id();
                let node = self.parse_alt();
                let _ = self.expect(')');
                RegexNode::Group {
                    id,
                    node: Box::new(node),
                }
            }
            Some('[') => self.parse_char_class(),
            Some('\\') => {
                // Parse escape sequences: \d, \w, \1, etc.
                self.advance();
                match self.advance() {
                    Some('d') => RegexNode::Digit, // \d matches a digit
                    Some('w') => RegexNode::Word,  // \w matches a word character
                    Some(c) if c.is_ascii_digit() && c != '0' => {
                        // \1, \2, ... are backreferences to group
                        let id = (c as u8 - b'0') as usize;
                        RegexNode::BackRef { id }
                    }
                    Some(c) => RegexNode::Literal(c), // Any other escaped char is literal
                    None => RegexNode::Literal('\\'), // Lone backslash at end
                }
            }
            Some('.') => {
                self.advance();
                RegexNode::Dot // . matches any character
            }
            Some('^') => {
                self.advance();
                RegexNode::StartAnchor // ^ matches start of input
            }
            Some('$') => {
                self.advance();
                RegexNode::EndAnchor // $ matches end of input
            }
            Some(c) => {
                self.advance();
                RegexNode::Literal(c) // Any other character is a literal
            }
            None => RegexNode::Seq(vec![]), // End of pattern
        }
    }

    /// Parse a character class, e.g. `[abc]` or `[^abc]`.
    ///
    /// Examples:
    /// - Pattern: `[abc]`   → CharClass { chars: ['a','b','c'], negated: false }
    /// - Pattern: `[^xyz]`  → CharClass { chars: ['x','y','z'], negated: true }
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
