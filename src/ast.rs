#[derive(Debug, Clone)]
pub enum RegexNode {
    Seq(Vec<RegexNode>),
    Alt(Vec<RegexNode>),
    Repeat {
        node: Box<RegexNode>,
        kind: RepeatKind,
    },
    Group {
        id: usize,
        node: Box<RegexNode>,
    },
    BackRef {
        id: usize,
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
pub enum RepeatKind {
    ZeroOrOne,
    OneOrMore,
}
