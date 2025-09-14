use crate::ast::{RegexNode, RepeatKind};

pub struct Parser<'a> {
    pub pattern: &'a str,
    pub pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(pattern: &'a str) -> Self {
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

    pub fn parse(&mut self) -> RegexNode {
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
                RegexNode::Repeat { node: Box::new(atom), kind: RepeatKind::ZeroOrOne }
            }
            Some('+') => {
                self.advance();
                RegexNode::Repeat { node: Box::new(atom), kind: RepeatKind::OneOrMore }
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
            Some('.') => { self.advance(); RegexNode::Dot }
            Some('^') => { self.advance(); RegexNode::StartAnchor }
            Some('$') => { self.advance(); RegexNode::EndAnchor }
            Some(c) => { self.advance(); RegexNode::Literal(c) }
            None => RegexNode::Seq(vec![]),
        }
    }

    fn parse_char_class(&mut self) -> RegexNode {
        let _ = self.advance(); // consume '['
        let negated = if self.peek() == Some('^') { self.advance(); true } else { false };
        let mut chars_in_class = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == ']' { break; }
            chars_in_class.push(self.advance().unwrap());
        }
        let _ = self.expect(']');
        RegexNode::CharClass { chars: chars_in_class, negated }
    }
}
