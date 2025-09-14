#[derive(Debug, Clone)]
pub enum RegexNode {
    Seq(Vec<RegexNode>), // Sequence of nodes, e.g. "abc"
    Alt(Vec<RegexNode>), // Alternation, e.g. "a|b"
    Repeat {
        node: Box<RegexNode>,
        kind: RepeatKind,
    }, // Repetition, e.g. "a?" or "a+"
    Group {
        id: usize,
        node: Box<RegexNode>,
    }, // Capturing group, e.g. "(abc)"
    BackRef {
        id: usize,
    }, // Backreference, e.g. "\1"
    StartAnchor,         // Start anchor, e.g. "^"
    EndAnchor,           // End anchor, e.g. "$"
    Dot,                 // Any character, e.g. "."
    Digit,               // Digit character, e.g. "\d"
    Word,                // Word character, e.g. "\w"
    CharClass {
        chars: Vec<char>,
        negated: bool,
    }, // Character class, e.g. "[abc]" or "[^abc]"
    Literal(char),       // Literal character, e.g. "a"
}

#[derive(Debug, Clone, Copy)]
pub enum RepeatKind {
    ZeroOrOne, // "?" quantifier, e.g. "a?"
    OneOrMore, // "+" quantifier, e.g. "a+"
}
