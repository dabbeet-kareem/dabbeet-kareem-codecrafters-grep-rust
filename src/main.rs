use std::env;
use std::io;
use std::process;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    match pattern {
        p if p.chars().count() == 1 => input_line.contains(p),
        // check if pattern start with [ and end with ]
        p if p.starts_with('[') && p.ends_with(']') => {
            let chars = p[1..p.len() - 1].chars().collect::<Vec<char>>();
            input_line.chars().any(|c| chars.contains(&c))
        }
        "\\d" => input_line.chars().any(|c| c.is_digit(10)),
        "\\w" => input_line.chars().any(|c| c.is_alphanumeric() || c == '_'),
        _ => panic!("Unhandled pattern: {}", pattern),
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

    if match_pattern(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
