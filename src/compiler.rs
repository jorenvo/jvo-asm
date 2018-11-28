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

struct InstructionMove<'a> {
    operation: &'a Token,
    left: &'a Token,
    right: &'a Token,
}

trait Compile {
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>>;
}

impl<'a> Compile for InstructionMove<'a> {
    fn compile(&self) -> Result<Vec<u8>, Box<error::Error>> {
        // TODO only supports moving immediate values for now
        let mut opcode = 0xb8;

        // register is specified in 3 LSb's
        opcode |= get_reg_value(self.right)?;

        Ok(vec![opcode, self.left.value.parse::<u8>().unwrap(), 0x00])
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
    fn test_immediate1() {
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
        assert!(vec_compare(&[0xba, 0x01, 0x00], &bytes));
    }

    #[test]
    fn test_immediate2() {
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
        assert!(vec_compare(&[0xb8, 0x00, 0x00], &bytes));
    }
}

pub fn compile(tokens: Vec<Token>) -> Result<Vec<u8>, Box<error::Error>> {
    let operation = match tokens[0].t {
        Some(TokenType::Move) => InstructionMove {
            operation: &tokens[0],  // TODO check if the token types are correct
            left: &tokens[1],
            right: &tokens[2],
        },

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

// Notes:
// Instruction format (p 505)
// We won't do:
// - stick to 1 byte opcodes
// - mod/rm optionally 1 byte
// - SIB optional
// 1B opcode, (1B MODR/M), (1B SIB), (1B displacement), (1B immediate)

// MOV (p 1161)
// opcode: 8b
// 3 least significant opcode bits are used to encode the register
// registers codes (p 574)
