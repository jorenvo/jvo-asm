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

fn get_reg_value(token: &Token) -> Result<u8, Box<error::Error>> {
    // p 574
    match token.value.as_str() {
        "eax" => Ok(0),
        "ecx" => Ok(1),
        "edx" => Ok(2),
        "ebx" => Ok(3),
        _ => Err(Box::new(CompileError {
            msg: format!("{} is not a valid register", token.value),
        })),
    }
}

trait Instruction {
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>>;
}

struct InstructionMove<'a> {
    operation: &'a Token,
    left: &'a Token,
    right: &'a Token,
}

impl<'a> Instruction for InstructionMove<'a> {
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        // p 1161
        // TODO only supports moving immediate values for now
        let mut opcode = 0xb8;

        // register is specified in 3 LSb's
        opcode |= get_reg_value(self.right)?;

        Ok(vec![
            opcode,
            self.left.value.parse::<u8>().unwrap(),
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
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        // p 1031
        Ok(vec![self.operand.value.parse::<u8>()?, 0xcd])
    }
}

#[cfg(test)]
mod test_instruction_move {
    use super::*;

    fn vec_compare(va: &[u8], vb: &[u8]) -> bool {
        (va.len() == vb.len()) &&  // zip stops at the shortest
            va.iter()
            .zip(vb)
            .all(|(a,b)| a == b)
    }

    #[test]
    fn test_move_immediate1() {
        let operation = Token {
            t: Some(TokenType::Move),
            value: "mov".to_string(),
        };
        let left = Token {
            t: Some(TokenType::Value),
            value: "1".to_string(),
        };
        let right = Token {
            t: Some(TokenType::Register),
            value: "edx".to_string(),
        };
        let instruction = InstructionMove {
            operation: &operation,
            left: &left,
            right: &right,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                0xb8 | get_reg_value(&right).unwrap(),
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
        let operation = Token {
            t: Some(TokenType::Move),
            value: "mov".to_string(),
        };
        let left = Token {
            t: Some(TokenType::Value),
            value: "0".to_string(),
        };
        let right = Token {
            t: Some(TokenType::Register),
            value: "eax".to_string(),
        };
        let instruction = InstructionMove {
            operation: &operation,
            left: &left,
            right: &right,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                0xb8 | get_reg_value(&right).unwrap(),
                0x00,
                0x00,
                0x00,
                0x00
            ],
            &bytes
        ));
    }

    #[test]
    fn test_interrupt_linux() {
        let operation = Token {
            t: Some(TokenType::Interrupt),
            value: "int".to_string(),
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
        assert!(vec_compare(&[128, 0xcd, 0x00, 0x00], &bytes));
    }
}

pub fn compile(tokens: Vec<Token>) -> Result<Vec<u8>, Box<error::Error>> {
    let operation: Box<Instruction> = match tokens[0].t {
        // TODO check if the token types are correct
        Some(TokenType::Move) => Box::new(InstructionMove {
            operation: &tokens[0],
            left: &tokens[1],
            right: &tokens[2],
        }),
        Some(TokenType::Interrupt) => Box::new(InstructionInterrupt {
            operation: &tokens[0],
            operand: &tokens[1],
        }),
        _ => {
            return Err(Box::new(CompileError {
                msg: format!(
                    "Grammatical error: {}",
                    tokens.iter().fold("".to_string(), |acc, t| acc.to_owned()
                        + &format!(" {}", t.value))
                ),
            }));
        }
    };

    operation.compile()
}
