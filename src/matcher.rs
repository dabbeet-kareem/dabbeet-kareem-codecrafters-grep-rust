use crate::ast::RegexNode;

// Return all possible end positions after matching `node` at `pos`.
pub fn match_node(node: &RegexNode, input: &[char], pos: usize) -> Vec<usize> {
    match node {
        RegexNode::Literal(c) => {
            if pos < input.len() && input[pos] == *c { vec![pos + 1] } else { vec![] }
        }
        RegexNode::Dot => {
            if pos < input.len() { vec![pos + 1] } else { vec![] }
        }
        RegexNode::Digit => {
            if pos < input.len() && input[pos].is_digit(10) { vec![pos + 1] } else { vec![] }
        }
        RegexNode::Word => {
            if pos < input.len() && (input[pos].is_alphanumeric() || input[pos] == '_') { vec![pos + 1] } else { vec![] }
        }
        RegexNode::CharClass { chars, negated } => {
            if pos >= input.len() { return vec![]; }
            let contains = chars.contains(&input[pos]);
            if (*negated && !contains) || (!*negated && contains) { vec![pos + 1] } else { vec![] }
        }
        RegexNode::StartAnchor => {
            if pos == 0 { vec![pos] } else { vec![] }
        }
        RegexNode::EndAnchor => {
            if pos == input.len() { vec![pos] } else { vec![] }
        }
        RegexNode::Seq(nodes) => {
            let mut positions = vec![pos];
            for n in nodes {
                let mut next_positions = Vec::new();
                for p in positions {
                    let res = match_node(n, input, p);
                    next_positions.extend(res);
                }
                if next_positions.is_empty() { return vec![]; }
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
            crate::ast::RepeatKind::ZeroOrOne => {
                let mut positions = vec![pos];
                positions.extend(match_node(inner, input, pos));
                positions.sort_unstable();
                positions.dedup();
                positions
            }
            crate::ast::RepeatKind::OneOrMore => {
                let mut results = Vec::new();
                let mut frontier = match_node(inner, input, pos);
                while !frontier.is_empty() {
                    for p in &frontier {
                        if !results.contains(p) { results.push(*p); }
                    }
                    let mut next = Vec::new();
                    for p in &frontier { next.extend(match_node(inner, input, *p)); }
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
