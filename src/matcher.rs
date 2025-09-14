use crate::ast::{RegexNode, RepeatKind};

#[derive(Clone, Debug)]
struct MatchState { pos: usize, captures: Vec<Option<(usize, usize)>> }

fn max_group_id(node: &RegexNode) -> usize {
    match node {
        RegexNode::Group { id, node } => (*id).max(max_group_id(node)),
        RegexNode::Seq(v) | RegexNode::Alt(v) => v.iter().map(max_group_id).max().unwrap_or(0),
        RegexNode::Repeat { node, .. } => max_group_id(node),
        _ => 0,
    }
}

fn match_with_state(node: &RegexNode, input: &[char], state: &MatchState) -> Vec<MatchState> {
    match node {
        RegexNode::Literal(c) => {
            if state.pos < input.len() && input[state.pos] == *c {
                let mut next = state.clone(); next.pos += 1; vec![next]
            } else {
                vec![]
            }
        }
        RegexNode::Dot => {
            if state.pos < input.len() {
                let mut next = state.clone(); next.pos += 1; vec![next]
            } else {
                vec![]
            }
        }
        RegexNode::Digit => {
            if state.pos < input.len() && input[state.pos].is_digit(10) {
                let mut next = state.clone(); next.pos += 1; vec![next]
            } else {
                vec![]
            }
        }
        RegexNode::Word => {
            if state.pos < input.len() && (input[state.pos].is_alphanumeric() || input[state.pos] == '_') {
                let mut next = state.clone(); next.pos += 1; vec![next]
            } else {
                vec![]
            }
        }
        RegexNode::CharClass { chars, negated } => {
            if state.pos >= input.len() {
                return vec![];
            }
            let contains = chars.contains(&input[state.pos]);
            if (*negated && !contains) || (!*negated && contains) {
                let mut next = state.clone(); next.pos += 1; vec![next]
            } else {
                vec![]
            }
        }
        RegexNode::StartAnchor => {
            if state.pos == 0 {
                vec![state.clone()]
            } else {
                vec![]
            }
        }
        RegexNode::EndAnchor => {
            if state.pos == input.len() {
                vec![state.clone()]
            } else {
                vec![]
            }
        }
        RegexNode::Group { id, node } => {
            let start = state.pos;
            let mut out = Vec::new();
            for mut ns in match_with_state(node, input, state) {
                let end = ns.pos;
                if *id < ns.captures.len() { ns.captures[*id] = Some((start, end)); }
                out.push(ns);
            }
            out
        }
        RegexNode::BackRef { id } => {
            if *id >= state.captures.len() { return vec![]; }
            if let Some((s, e)) = state.captures[*id] {
                let len = e - s;
                if state.pos + len <= input.len() && input[s..e] == input[state.pos..state.pos+len] {
                    let mut next = state.clone(); next.pos += len; vec![next]
                } else { vec![] }
            } else { vec![] }
        }
        RegexNode::Seq(nodes) => {
            let mut states = vec![state.clone()];
            for n in nodes {
                let mut next_states = Vec::new();
                for st in &states {
                    next_states.extend(match_with_state(n, input, st));
                }
                if next_states.is_empty() {
                    return vec![];
                }
                next_states.sort_by_key(|s| s.pos);
                next_states.dedup_by_key(|s| s.pos);
                states = next_states;
            }
            states
        }
        RegexNode::Alt(branches) => {
            let mut all_states = Vec::new();
            for br in branches {
                all_states.extend(match_with_state(br, input, state));
            }
            all_states.sort_by_key(|s| s.pos);
            all_states.dedup_by_key(|s| s.pos);
            all_states
        }
        RegexNode::Repeat { node: inner, kind } => match kind {
            RepeatKind::ZeroOrOne => {
                let mut out = vec![state.clone()];
                out.extend(match_with_state(inner, input, state));
                out.sort_by_key(|s| s.pos);
                out.dedup_by_key(|s| s.pos);
                out
            }
            RepeatKind::OneOrMore => {
                let mut results: Vec<MatchState> = Vec::new();
                let mut frontier = match_with_state(inner, input, state);
                while !frontier.is_empty() {
                    for st in &frontier {
                        if !results.iter().any(|r| r.pos == st.pos) { results.push(st.clone()); }
                    }
                    let mut next = Vec::new();
                    for st in &frontier { next.extend(match_with_state(inner, input, st)); }
                    next.sort_by_key(|s| s.pos);
                    next.dedup_by_key(|s| s.pos);
                    frontier = next;
                }
                results
            }
        },
    }
}

pub fn match_node(node: &RegexNode, input: &[char], start: usize) -> Vec<usize> {
    let max_id = max_group_id(node);
    let init = MatchState { pos: start, captures: vec![None; max_id + 1] };
    match_with_state(node, input, &init).into_iter().map(|s| s.pos).collect()
}
