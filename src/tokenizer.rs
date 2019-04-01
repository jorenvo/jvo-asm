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
use common::*;
use std::{error, fmt};

#[derive(Debug, Clone)]
struct TokenizeError {
    msg: String,
}

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for TokenizeError {
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
        "↩" => {
            token.t = Some(TokenType::Return);
        }
        "⬆" => {
            token.t = Some(TokenType::Add);
        }
        "➖" => {
            token.t = Some(TokenType::Subtract);
        }
        "✖" => {
            token.t = Some(TokenType::Multiply);
        }
        "⬅" => {
            token.t = Some(TokenType::Move);
        }
        "❗" => {
            token.t = Some(TokenType::Interrupt);
        }
        "⚪" | "🔴" | "🔵" | "⚫" | "◀" | "⬇" => {
            token.t = Some(TokenType::Register);
        }
        "🦘=" => {
            token.t = Some(TokenType::JumpIfEqual);
        }
        "🦘≠" => {
            token.t = Some(TokenType::JumpIfNotEqual);
        }
        "🦘<" => {
            token.t = Some(TokenType::JumpIfLess);
        }
        "🦘≤" => {
            token.t = Some(TokenType::JumpIfLessEqual);
        }
        "🦘>" => {
            token.t = Some(TokenType::JumpIfGreater);
        }
        "🦘≥" => {
            token.t = Some(TokenType::JumpIfGreaterEqual);
        }
        "🦘" => {
            token.t = Some(TokenType::Jump);
        }
        "📞" => {
            token.t = Some(TokenType::Call);
        }
        "🏠" => {
            token.t = Some(TokenType::Return);
        }
        "📥" => {
            token.t = Some(TokenType::Push);
        }
        "📤" => {
            token.t = Some(TokenType::Pop);
        }
        "⚖" => {
            token.t = Some(TokenType::Compare);
        }
        _ if word.starts_with("🖊") => {
            token.t = Some(TokenType::Constant);
            token.value.remove(0);
        }
        _ if word.starts_with("📪") && word.ends_with(':') => {
            token.t = Some(TokenType::Label);

            let to_trim: &[_] = &[':', '📪'];
            token.value = token.value.trim_matches(to_trim).to_string();
        }
        _ if word.starts_with("📗") => {
            token.t = Some(TokenType::Section);
            token.value.remove(0);
        }
        _ if word.starts_with("✉") => {
            token.t = Some(TokenType::LabelReference);
            token.value.remove(0);
        }
        _ if word.starts_with('$') => {
            if word[1..].parse::<u32>().is_err() {
                return Err(Box::new(TokenizeError {
                    msg: format!("Invalid value: {}. Should be a number.", word),
                }));
            }

            token.t = Some(TokenType::Value);
            token.value.remove(0);
        }
        _ if word.parse::<u32>().is_ok() => {
            token.t = Some(TokenType::Memory);
        }
        _ => {
            token.t = Some(TokenType::ConstantReference);
        }
    };

    Ok(token)
}

pub fn tokenize(line: &str) -> Result<Vec<Token>, Box<error::Error>> {
    let mut tokens = vec![];
    let ignore_char = |c: char| c == ',' || c.is_whitespace();
    let is_delimiter = |c: char| c == ' ' || c == '~';

    for word in line.split(is_delimiter) {
        let word = word.trim_matches(ignore_char);
        if word.is_empty() {
            continue;
        }

        if word.starts_with('#') {
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
    fn test_const_reference() {
        let tokens = tokenize("foobar").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::ConstantReference));
    }

    fn verify_ret(tokens: &Vec<Token>) {
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::Return));
    }

    #[test]
    fn test_ret() {
        let tokens = tokenize("↩").unwrap();
        verify_ret(&tokens);
    }

    fn verify_add(tokens: &Vec<Token>) {
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Register));
        assert_eq!(tokens[0].value, "⚪");

        assert_eq!(tokens[1].t, Some(TokenType::Add));

        assert_eq!(tokens[2].t, Some(TokenType::Value));
        assert_eq!(tokens[2].value, "5");
    }

    #[test]
    fn test_add() {
        let tokens = tokenize("⚪ ⬆ $5").unwrap();
        verify_add(&tokens);
    }

    #[test]
    fn test_subtract() {
        let tokens = tokenize("⚪ ➖ $5").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Register));
        assert_eq!(tokens[0].value, "⚪");

        assert_eq!(tokens[1].t, Some(TokenType::Subtract));

        assert_eq!(tokens[2].t, Some(TokenType::Value));
        assert_eq!(tokens[2].value, "5");
    }

    #[test]
    fn test_multiply() {
        let tokens = tokenize("⚪ ✖ $5").unwrap();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Register));
        assert_eq!(tokens[0].value, "⚪");

        assert_eq!(tokens[1].t, Some(TokenType::Multiply));

        assert_eq!(tokens[2].t, Some(TokenType::Value));
        assert_eq!(tokens[2].value, "5");
    }

    #[test]
    fn test_memory() {
        let tokens = tokenize("321").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::Memory));
        assert_eq!(tokens[0].value, "321");
    }

    #[test]
    fn test_label() {
        let tokens = tokenize("📪my_label:").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::Label));
        assert_eq!(tokens[0].value, "my_label");
    }

    #[test]
    fn test_jump() {
        let tokens = tokenize("🦘 123").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].t, Some(TokenType::Jump));
        assert_eq!(tokens[1].t, Some(TokenType::Memory));
        assert_eq!(tokens[1].value, "123");
    }

    #[test]
    fn test_jump_if_condition() {
        let tokens = tokenize("🦘≠ 123").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].t, Some(TokenType::JumpIfNotEqual));
        assert_eq!(tokens[0].value, "🦘≠");
        assert_eq!(tokens[1].t, Some(TokenType::Memory));
        assert_eq!(tokens[1].value, "123");
    }

    #[test]
    fn test_call() {
        let tokens = tokenize("📞 123").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].t, Some(TokenType::Call));
        assert_eq!(tokens[1].t, Some(TokenType::Memory));
        assert_eq!(tokens[1].value, "123");
    }

    #[test]
    fn test_return() {
        let tokens = tokenize("🏠").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].t, Some(TokenType::Return));
    }

    #[test]
    fn test_compare_registers() {
        let tokens = tokenize("⚖ ⚪ ⚫").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Compare));
        assert_eq!(tokens[1].t, Some(TokenType::Register));
        assert_eq!(tokens[2].t, Some(TokenType::Register));
    }

    #[test]
    fn test_whitespace1() {
        let tokens = tokenize("↩        ").unwrap();
        verify_ret(&tokens);
    }

    #[test]
    fn test_whitespace2() {
        let tokens = tokenize("    ↩        ").unwrap();
        verify_ret(&tokens);
    }

    #[test]
    fn test_whitespace3() {
        let tokens = tokenize("⚪ 	⬆ $5").unwrap();
        //                        ^ TAB
        verify_add(&tokens);
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("↩ # some comment").unwrap();
        verify_ret(&tokens);
    }

    #[test]
    fn test_invalid_value() {
        let tokens = tokenize("⚪ ⬅ $SYS_EXIT");
        assert!(tokens.is_err());
    }

    #[test]
    fn test_full_line_comment() {
        let tokens = tokenize("# ↩ some comment").unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_constant_offset() {
        let tokens = tokenize("📥 offset~⬇").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].t, Some(TokenType::Push));
        assert_eq!(tokens[1].t, Some(TokenType::ConstantReference));
        assert_eq!(tokens[2].t, Some(TokenType::Register));
    }
}
