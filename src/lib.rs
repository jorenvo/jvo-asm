// Copyright 2018, Joren Van Onder (joren.vanonder@gmail.com)
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
#![allow(unused)]
use std::{error, fmt, fs};

pub struct Config {
    pub filename: String,
}

impl Config {
    pub fn new(mut args: Vec<String>) -> Result<Config, String> {
        let program_name = args.remove(0);

        if args.len() < 1 {
            Err(format!("Usage: {} program.jas", program_name))
        } else {
            Ok(Config {
                filename: args.remove(0),
            })
        }
    }
}

// enum Instruction {
//     Move,
//     Add,
// }

// struct Move {
//     left: Token,
//     right: Token,
// }

// struct Add {
//     left: Token,
//     right: Token,
// }

// struct Value {}
// struct Memory {}
// struct BasePointerMemory {}
// struct Register {}

#[derive(Debug, PartialEq)]
enum TokenType {
    Move,
    Add,
    Value,
    Memory,
    BasePointerMemory,
    Register,
    Return,
}

struct Token {
    t: Option<TokenType>,
    value: String,
}

#[derive(Debug, Clone)]
struct CompileError {
    msg: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for CompileError {
    fn description(&self) -> &str {
        self.msg.as_str()
    }
}

fn tokenize_word(word: &str) -> Result<Token, Box<error::Error>> {
    let mut token = Token {
        t: None,
        value: word.to_string(),
    };

    match word {
        "ret" => {
            token.t = Some(TokenType::Return);
        }
        "add" => {
            token.t = Some(TokenType::Add);
        }
        _ if word.starts_with("$") => {
            token.t = Some(TokenType::Value);
        }
        _ if word.starts_with("%") => {
            token.t = Some(TokenType::Register);
        }
        _ => {
            return Err(Box::new(CompileError {
                msg: format!("Unexpected token: {}", word),
            }));
        }
    };

    return Ok(token);
}

fn tokenize(line: &str) -> Result<Vec<Token>, Box<error::Error>> {
    let mut tokens = vec![];
    let ignore_char = |c: char| c == ',' || c.is_whitespace();

    for word in line.split(' ') {
        let word = word.trim_matches(ignore_char);
        if word.is_empty() {
            continue;
        }

        if word.starts_with("#") {
            break;
        }

        let token = tokenize_word(word)?;
        tokens.push(token);
    }

    Ok(tokens)
}

#[cfg(test)]
mod test_tokenize {
    use super::*;

    #[test]
    fn test_empty_string() {
        let tokens = tokenize("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_syntax_error1() {
        assert!(tokenize("foobar").is_err());
    }

    #[test]
    fn test_syntax_error2() {
        assert!(tokenize("add $5, x").is_err());
    }

    fn verify_ret(tokens: &Vec<Token>) {
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::Return));
    }

    #[test]
    fn test_ret() {
        let tokens = tokenize("ret").unwrap();
        verify_ret(&tokens);
    }

    fn verify_add(tokens: &Vec<Token>) {
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Add));

        assert_eq!(tokens[1].t, Some(TokenType::Value));
        assert_eq!(tokens[1].value, "$5");

        assert_eq!(tokens[2].t, Some(TokenType::Register));
        assert_eq!(tokens[2].value, "%eax");
    }

    #[test]
    fn test_add() {
        let tokens = tokenize("add $5, %eax").unwrap();
        verify_add(&tokens);
    }

    #[test]
    fn test_whitespace1() {
        let tokens = tokenize("ret        ").unwrap();
        verify_ret(&tokens);
    }

    #[test]
    fn test_whitespace2() {
        let tokens = tokenize("    ret        ").unwrap();
        verify_ret(&tokens);
    }

    #[test]
    fn test_whitespace3() {
        let tokens = tokenize("add 	$5   ,    %eax").unwrap();
        //                         ^ TAB
        verify_add(&tokens);
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("ret # some comment").unwrap();
        verify_ret(&tokens);
    }
}

fn compile(tokens: Vec<Token>) -> Result<Vec<u8>, Box<error::Error>> {
    Err(Box::new(CompileError {
        msg: "Not implemented!".to_string(),
    }))
}

fn process(filename: &str) -> Result<Vec<u8>, Box<error::Error>> {
    let mut program: Vec<u8> = vec![];
    let content = fs::read_to_string(filename)?;

    for line in content.split('\n') {
        let tokens = tokenize(line)?;
        let mut bytes = compile(tokens)?;

        program.append(&mut bytes);
    }

    Ok(program)
}

pub fn run(config: Config) {
    println!("compile {}", config.filename);

    let program = process(&config.filename).unwrap();
}
