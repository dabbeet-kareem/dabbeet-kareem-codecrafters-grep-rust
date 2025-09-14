pub mod ast;
pub mod matcher;
pub mod parser;

pub fn is_match(input: &str, pattern: &str) -> bool {
    let mut p = parser::Parser::new(pattern);
    let ast = p.parse();
    let chars: Vec<char> = input.chars().collect();
    for start in 0..=chars.len() {
        if !matcher::match_node(&ast, &chars, start).is_empty() {
            return true;
        }
    }
    false
}
