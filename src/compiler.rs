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

// Notes:
// Instruction format (p 505)

#![allow(unused_variables, dead_code)]
use common::*;
use std::{error, fmt};

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

// struct Value {}
// struct Memory {}
// struct BasePointerMemory {}
// struct Register {}

fn get_reg_value(token: &Token) -> Result<u8, Box<error::Error>> {
    // p 574
    match token.value.as_str() {
        "⚪" => Ok(0),  // eax
        "🔵" => Ok(1), // ecx
        "⚫" => Ok(2),  // edx
        "🔴" => Ok(3), // ebx
        _ => Err(Box::new(CompileError {
            msg: format!("{} is not a valid register", token.value),
        })),
    }
}

trait Instruction {
    fn validate(&self) -> Result<(), Box<error::Error>>;
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>>;

    fn format_tokens(&self, tokens: &Vec<&Token>) -> String {
        tokens.iter().fold("".to_string(), |acc, t| {
            acc.to_owned() + &format!(" {}", t.value)
        })
    }

    fn validate_tokens(
        &self,
        expected: Vec<TokenType>,
        given: Vec<&Token>,
    ) -> Result<(), Box<error::Error>> {
        // this shouldn't happen because the compiler already created
        // the instruction before and probably dropped any excess
        // tokens.
        if expected.len() != given.len() {
            return Err(Box::new(CompileError {
                msg: format!(
                    "Grammatical error: {}, incorrect amount of tokens",
                    self.format_tokens(&given),
                ),
            }));
        }

        for (expected_token, given_token) in expected.iter().zip(given.iter()) {
            if let Some(ref given_token_t) = given_token.t {
                if expected_token != given_token_t {
                    return Err(Box::new(CompileError {
                        msg: format!(
                            "Grammatical error: {}, {} should be a {:?}.",
                            self.format_tokens(&given),
                            given_token,
                            expected_token,
                        ),
                    }));
                }
            } else {
                return Err(Box::new(CompileError {
                    msg: format!(
                        "Grammatical error: {}, expected a {:?}",
                        self.format_tokens(&given),
                        expected_token,
                    ),
                }));
            }
        }

        Ok(())
    }
}

struct InstructionMove<'a> {
    register: &'a Token,
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionMove<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![TokenType::Register, TokenType::Move, TokenType::Value],
            vec![&self.register, &self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        self.validate()?;
        // p 1161
        // TODO only supports moving immediate values for now
        let mut opcode = 0xb8;

        // register is specified in 3 LSb's
        opcode |= get_reg_value(self.register)?;

        Ok(vec![
            opcode,
            self.operand.value.parse::<u8>().unwrap(),
            0x00,
            0x00,
            0x00,
        ])
    }
}

struct InstructionAdd<'a> {
    register: &'a Token,
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionAdd<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![TokenType::Register, TokenType::Add, TokenType::Value],
            vec![&self.register, &self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        self.validate()?;
        // modr/m p507, p513
        // p603
        // todo currently only supports adding immediate values
        let mod_ = 0b11000000;
        let reg = 0;
        let rm = get_reg_value(&self.register).unwrap();

        Ok(vec![
            0x81, // 32 bit adds
            mod_ | reg | rm,
            self.operand.value.parse::<u8>()?, // todo support >1 byte
            0x00,
            0x00,
            0x00,
        ])
    }
}

struct InstructionInterrupt<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionInterrupt<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![TokenType::Interrupt, TokenType::Value],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        self.validate()?;
        // p 1031
        Ok(vec![
            0xcd,
            self.operand.value.parse::<u8>()?,
            0x00,
            0x00,
            0x00,
        ])
    }
}

#[cfg(test)]
mod test_instructions {
    use super::*;

    fn vec_compare(va: &[u8], vb: &[u8]) -> bool {
        (va.len() == vb.len()) &&  // zip stops at the shortest
            va.iter()
            .zip(vb)
            .all(|(a,b)| a == b)
    }

    #[test]
    fn test_move_immediate1() {
        let register = Token {
            t: Some(TokenType::Register),
            value: "⚫".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Move),
            value: "⬅".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "1".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                0xb8 | get_reg_value(&register).unwrap(),
                0x01,
                0x00,
                0x00,
                0x00
            ],
            &bytes
        ));
    }

    #[test]
    fn test_move_immediate2() {
        let register = Token {
            t: Some(TokenType::Register),
            value: "⚪".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Move),
            value: "⬅".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "0".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                0xb8 | get_reg_value(&register).unwrap(),
                0x00,
                0x00,
                0x00,
                0x00
            ],
            &bytes
        ));
    }

    #[test]
    fn test_add_immediate1() {
        let register = Token {
            t: Some(TokenType::Register),
            value: "⚫".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Add),
            value: "⬆".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "7".to_string(),
        };
        let instruction = InstructionAdd {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                0x81,
                0b11000000 | get_reg_value(&register).unwrap(),
                0x07,
                0x00,
                0x00,
                0x00,
            ],
            &bytes
        ));
    }

    #[test]
    fn test_interrupt_linux() {
        let operation = Token {
            t: Some(TokenType::Interrupt),
            value: "❗".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "128".to_string(),
        };
        let instruction = InstructionInterrupt {
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(&[0xcd, 128, 0x00, 0x00, 0x00], &bytes));
    }

    #[test]
    fn test_interrupt_validate_ok() {
        let operation = Token {
            t: Some(TokenType::Interrupt),
            value: "❗".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "$123".to_string(),
        };
        let instruction = InstructionInterrupt {
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_interrupt_validate_error() {
        let operation = Token {
            t: Some(TokenType::Interrupt),
            value: "❗".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Add),
            value: "️".to_string(),
        };
        let instruction = InstructionInterrupt {
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_interrupt_validate_error2() {
        let operation = Token {
            t: Some(TokenType::Interrupt),
            value: "❗".to_string(),
        };
        let operand = Token {
            t: None,
            value: "".to_string(),
        };
        let instruction = InstructionInterrupt {
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_move_validate_ok() {
        let register = Token {
            t: Some(TokenType::Register),
            value: "⚪".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Move),
            value: "⬅".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "0".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_move_validate_err() {
        let register = Token {
            t: Some(TokenType::Value),
            value: "123".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Move),
            value: "⬅".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "0".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_err());
    }
}

pub fn compile(tokens: Vec<Token>) -> Result<Vec<u8>, Box<error::Error>> {
    let mut operation: Option<Box<Instruction>> = None;

    for token in tokens.iter() {
        if token.t == Some(TokenType::Move) {
            operation = Some(Box::new(InstructionMove {
                register: &tokens[0],
                operation: &tokens[1],
                operand: &tokens[2],
            }));
            break;
        } else if token.t == Some(TokenType::Interrupt) {
            operation = Some(Box::new(InstructionInterrupt {
                operation: &tokens[0],
                operand: &tokens[1],
            }));
            break;
        } else if token.t == Some(TokenType::Add) {
            operation = Some(Box::new(InstructionAdd {
                register: &tokens[0],
                operation: &tokens[1],
                operand: &tokens[2],
            }));
            break;
        }
    }

    if operation.is_some() {
        let operation = operation.unwrap();
        operation.compile()
    } else {
        Err(Box::new(CompileError {
            msg: format!(
                "Grammatical error: {}, expected instruction",
                tokens.iter().fold("".to_string(), |acc, t| acc.to_owned()
                    + &format!(" {}", t.value))
            ),
        }))
    }
}
