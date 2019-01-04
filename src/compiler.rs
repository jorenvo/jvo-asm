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

fn get_reg_value(token: &Token) -> Result<u8, Box<error::Error>> {
    // p 574
    match token.value.as_str() {
        "⚪" => Ok(0),  // eax
        "🔵" => Ok(1), // ecx
        "⚫" => Ok(2),  // edx
        "🔴" => Ok(3), // ebx
        "◀" => Ok(4),  // esp
        "⬇" => Ok(5),  // ebp
        _ => Err(Box::new(CompileError {
            msg: format!("{} is not a valid register", token.value),
        })),
    }
}

trait Instruction {
    fn validate(&self) -> Result<(), Box<error::Error>>;
    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>>;

    fn format_tokens(&self, tokens: &Vec<&Token>) -> String {
        tokens.iter().fold("".to_string(), |acc, t| {
            acc.to_owned() + &format!(" {}", t.value)
        })
    }

    fn validate_tokens(
        &self,
        expected: Vec<Vec<TokenType>>,
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

        for (expected_tokens, given_token) in expected.iter().zip(given.iter()) {
            if let Some(ref given_token_t) = given_token.t {
                if !expected_tokens.contains(given_token_t) {
                    return Err(Box::new(CompileError {
                        msg: format!(
                            "Grammatical error: {}, {} should be a {:?}.",
                            self.format_tokens(&given),
                            given_token,
                            expected_tokens,
                        ),
                    }));
                }
            } else {
                return Err(Box::new(CompileError {
                    msg: format!(
                        "Grammatical error: {}, expected a {:?}",
                        self.format_tokens(&given),
                        expected_tokens,
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
            vec![
                vec![TokenType::Register],
                vec![TokenType::Move],
                vec![TokenType::Value, TokenType::Register],
            ],
            vec![&self.register, &self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        // p 1161
        match self.operand.t {
            Some(TokenType::Value) => {
                let mut opcode = 0xb8;
                // register is specified in 3 LSb's
                opcode |= get_reg_value(self.register)?;
                let value = serialize_le(self.operand.value.parse::<u32>()?);

                Ok(vec![
                    IntermediateCode::Byte(opcode),
                    IntermediateCode::Byte(value[0]),
                    IntermediateCode::Byte(value[1]),
                    IntermediateCode::Byte(value[2]),
                    IntermediateCode::Byte(value[3]),
                ])
            }
            // TokenType::Register
            _ => {
                let opcode = 0x89;
                let mod_ = 0b11000000;
                let reg = get_reg_value(&self.operand).unwrap();
                let rm = get_reg_value(&self.register).unwrap();

                Ok(vec![
                    IntermediateCode::Byte(opcode),
                    IntermediateCode::Byte(mod_ | reg << 3 | rm),
                ])
            }
        }
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
            vec![
                vec![TokenType::Register],
                vec![TokenType::Add],
                vec![TokenType::Value],
            ],
            vec![&self.register, &self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        // modr/m p507, p513
        // p603
        // todo currently only supports adding immediate values
        let mod_ = 0b11000000;
        let reg = 0;
        let rm = get_reg_value(&self.register).unwrap();
        let value = serialize_le(self.operand.value.parse::<u32>()?);

        Ok(vec![
            IntermediateCode::Byte(0x81), // 32 bit adds
            IntermediateCode::Byte(mod_ | reg | rm),
            IntermediateCode::Byte(value[0]),
            IntermediateCode::Byte(value[1]),
            IntermediateCode::Byte(value[2]),
            IntermediateCode::Byte(value[3]),
        ])
    }
}

struct InstructionJump<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionJump<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![vec![TokenType::Jump], vec![TokenType::LabelReference]],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        // p 1063
        // p 87 specifying an offset
        Ok(vec![
            IntermediateCode::Byte(0xe9),
            IntermediateCode::Displacement32(self.operand.value.clone()),
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
            vec![vec![TokenType::Interrupt], vec![TokenType::Value]],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        // p 1031
        Ok(vec![
            IntermediateCode::Byte(0xcd),
            IntermediateCode::Byte(self.operand.value.parse::<u8>()?),
        ])
    }
}

struct InstructionPush<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionPush<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![vec![TokenType::Push], vec![TokenType::Value]],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        let value = serialize_le(self.operand.value.parse::<u32>()?);
        // p 1633
        Ok(vec![
            IntermediateCode::Byte(0x68),
            IntermediateCode::Byte(value[0]),
            IntermediateCode::Byte(value[1]),
            IntermediateCode::Byte(value[2]),
            IntermediateCode::Byte(value[3]),
        ])
    }
}

struct InstructionPop<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionPop<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![vec![TokenType::Pop], vec![TokenType::Register]],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        // p 1633
        let opcode = 0x58 | get_reg_value(&self.operand).unwrap();
        Ok(vec![IntermediateCode::Byte(opcode)])
    }
}

struct InstructionCompare<'a> {
    operation: &'a Token,
    left_operand: &'a Token,
    right_operand: &'a Token,
}

impl<'a> Instruction for InstructionCompare<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![
                vec![TokenType::Compare],
                vec![TokenType::Register],
                vec![TokenType::Register],
            ],
            vec![&self.operation, &self.left_operand, &self.right_operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        let opcode = 0x39;
        let mod_ = 0b11000000;

        // Contrary to convention the order of these operands is more
        // in line with what you would expect. With x < y:
        // cmp x, y
        // will make jle jump.
        //
        // In a conventional assembly language with AT&T syntax this
        // would make jge jump, because there the second argument is
        // compared to the first.
        let reg = get_reg_value(&self.left_operand).unwrap();;
        let rm = get_reg_value(&self.right_operand).unwrap();

        // p 725
        Ok(vec![
            IntermediateCode::Byte(opcode),
            IntermediateCode::Byte(mod_ | reg << 3 | rm),
        ])
    }
}

#[cfg(test)]
mod test_instructions {
    use super::*;

    fn vec_compare(va: &[IntermediateCode], vb: &[IntermediateCode]) -> bool {
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
                IntermediateCode::Byte(0xb8 | get_reg_value(&register).unwrap()),
                IntermediateCode::Byte(0x01),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
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
                IntermediateCode::Byte(0xb8 | get_reg_value(&register).unwrap()),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
            ],
            &bytes
        ));
    }

    #[test]
    fn test_move_immediate3() {
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
            value: "4294967294".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0xb8 | get_reg_value(&register).unwrap()),
                IntermediateCode::Byte(0xfe),
                IntermediateCode::Byte(0xff),
                IntermediateCode::Byte(0xff),
                IntermediateCode::Byte(0xff),
            ],
            &bytes
        ));
    }

    #[test]
    fn test_move_register1() {
        let register = Token {
            t: Some(TokenType::Register),
            value: "🔵".to_string(),
        };
        let operation = Token {
            t: Some(TokenType::Move),
            value: "⬅".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Register),
            value: "◀".to_string(),
        };
        let instruction = InstructionMove {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[IntermediateCode::Byte(0x89), IntermediateCode::Byte(0xe1),],
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
                IntermediateCode::Byte(0x81),
                IntermediateCode::Byte(0b11000000 | get_reg_value(&register).unwrap()),
                IntermediateCode::Byte(0x07),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
                IntermediateCode::Byte(0x00),
            ],
            &bytes
        ));
    }

    #[test]
    fn test_add_immediate2() {
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
            value: "4294967294".to_string(),
        };
        let instruction = InstructionAdd {
            register: &register,
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0x81),
                IntermediateCode::Byte(0b11000000 | get_reg_value(&register).unwrap()),
                IntermediateCode::Byte(0xfe),
                IntermediateCode::Byte(0xff),
                IntermediateCode::Byte(0xff),
                IntermediateCode::Byte(0xff),
            ],
            &bytes
        ));
    }

    #[test]
    fn test_jump() {
        let operation = Token {
            t: Some(TokenType::Jump),
            value: "🦘".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::LabelReference),
            value: "test_label".to_string(),
        };
        let instruction = InstructionJump {
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0xe9),
                IntermediateCode::Displacement32("test_label".to_string())
            ],
            &bytes
        ));
    }

    #[test]
    fn test_push1() {
        let operation = Token {
            t: Some(TokenType::Push),
            value: "📥".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "174285409".to_string(),
        };
        let instruction = InstructionPush {
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0x68),
                IntermediateCode::Byte(0x61),
                IntermediateCode::Byte(0x62),
                IntermediateCode::Byte(0x63),
                IntermediateCode::Byte(0x0a),
            ],
            &bytes
        ));
    }

    #[test]
    fn test_pop1() {
        let operation = Token {
            t: Some(TokenType::Pop),
            value: "📤".to_string(),
        };
        let register = Token {
            t: Some(TokenType::Register),
            value: "⬇".to_string(),
        };
        let instruction = InstructionPop {
            operation: &operation,
            operand: &register,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(&[IntermediateCode::Byte(0x5d),], &bytes));
    }

    #[test]
    fn test_compare1() {
        let operation = Token {
            t: Some(TokenType::Compare),
            value: "⚖".to_string(),
        };
        let left_operand = Token {
            t: Some(TokenType::Register),
            value: "🔴".to_string(),
        };
        let right_operand = Token {
            t: Some(TokenType::Register),
            value: "⚪".to_string(),
        };
        let instruction = InstructionCompare {
            operation: &operation,
            left_operand: &left_operand,
            right_operand: &right_operand,
        };

        let bytes = instruction.compile().unwrap();
        println!("{:?}", bytes);
        assert!(vec_compare(
            &[IntermediateCode::Byte(0x39), IntermediateCode::Byte(0xd8)],
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
        assert!(vec_compare(
            &[IntermediateCode::Byte(0xcd), IntermediateCode::Byte(128),],
            &bytes
        ));
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
    fn test_push_validate() {
        let operation = Token {
            t: Some(TokenType::Push),
            value: "📥".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "123".to_string(),
        };
        let instruction = InstructionPush {
            operation: &operation,
            operand: &operand,
        };

        let result = instruction.validate();
        assert!(result.is_ok());
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

pub fn compile(tokens: Vec<Token>) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
    let mut operation: Option<Box<Instruction>> = None;

    for token in tokens.iter() {
        operation = match token.t {
            Some(TokenType::Move) => Some(Box::new(InstructionMove {
                register: &tokens[0],
                operation: &tokens[1],
                operand: &tokens[2],
            })),
            Some(TokenType::Interrupt) => Some(Box::new(InstructionInterrupt {
                operation: &tokens[0],
                operand: &tokens[1],
            })),
            Some(TokenType::Add) => Some(Box::new(InstructionAdd {
                register: &tokens[0],
                operation: &tokens[1],
                operand: &tokens[2],
            })),
            Some(TokenType::Jump) => Some(Box::new(InstructionJump {
                operation: &tokens[0],
                operand: &tokens[1],
            })),
            Some(TokenType::Push) => Some(Box::new(InstructionPush {
                operation: &tokens[0],
                operand: &tokens[1],
            })),
            Some(TokenType::Compare) => Some(Box::new(InstructionCompare {
                operation: &tokens[0],
                left_operand: &tokens[1],
                right_operand: &tokens[2],
            })),
            _ => None,
        };

        if operation.is_some() {
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
