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
use std::collections::HashSet;
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

trait Instruction {
    fn validate(&self) -> Result<(), Box<error::Error>>;
    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>>;

    fn format_tokens(&self, tokens: &Vec<&Token>) -> String {
        tokens.iter().fold("".to_string(), |acc, t| {
            acc.to_owned() + &format!(" {}", t.value)
        })
    }

    fn get_reg_value(&self, token: &Token) -> Result<u8, Box<error::Error>> {
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

    fn validate_tokens(
        &self,
        expected: Vec<HashSet<TokenType>>,
        given: Vec<&Token>,
    ) -> Result<(), Box<error::Error>> {
        // This shouldn't happen because the compiler already created
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

    fn calc_modrm(&self, mod_: u8, reg_opcode: u8, rm: u8) -> u8 {
        const MOD_SIZE: u32 = 2;
        const REG_OPCODE_SIZE: u32 = 3;
        const RM_SIZE: u32 = 3;
        let msg =
            |name, size, value| format!("{} should be {} bits but is {:#b}", name, size, value);

        // modr/m p507, p513, p603
        if (mod_ >> MOD_SIZE) > 0 {
            panic!(msg("mod", MOD_SIZE, mod_));
        }

        if (reg_opcode >> REG_OPCODE_SIZE) > 0 {
            panic!(msg("reg_opcode", REG_OPCODE_SIZE, reg_opcode));
        }

        if (rm >> RM_SIZE) > 0 {
            panic!(msg("rm", RM_SIZE, rm));
        }

        mod_ << 6 | reg_opcode << 3 | rm
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
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
                vec![TokenType::Move].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Value, TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
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
                opcode |= self.get_reg_value(self.register)?;
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
                let modrm = self.calc_modrm(
                    0b11,
                    self.get_reg_value(&self.operand).unwrap(),
                    self.get_reg_value(&self.register).unwrap(),
                );

                Ok(vec![
                    IntermediateCode::Byte(opcode),
                    IntermediateCode::Byte(modrm),
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
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
                vec![TokenType::Add].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Value].into_iter().collect::<HashSet<_>>(),
            ],
            vec![&self.register, &self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;
        let value = serialize_le(self.operand.value.parse::<u32>()?);
        let modrm = self.calc_modrm(0b11, 0, self.get_reg_value(&self.register).unwrap());

        Ok(vec![
            IntermediateCode::Byte(0x81), // 32 bit adds
            IntermediateCode::Byte(modrm),
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
            vec![
                vec![TokenType::Jump].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::LabelReference]
                    .into_iter()
                    .collect::<HashSet<_>>(),
            ],
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
            vec![
                vec![TokenType::Interrupt]
                    .into_iter()
                    .collect::<HashSet<_>>(),
                vec![TokenType::Value].into_iter().collect::<HashSet<_>>(),
            ],
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

struct InstructionPushImmediate<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionPushImmediate<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![
                vec![TokenType::Push].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Value].into_iter().collect::<HashSet<_>>(),
            ],
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

struct InstructionPushModRM<'a> {
    operation: &'a Token,
    offset: &'a Token,
    register: &'a Token,
}

impl<'a> Instruction for InstructionPushModRM<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![
                vec![TokenType::Push].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Value].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
            ],
            vec![&self.operation, &self.offset, &self.register],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        let opcode = 0xff;

        let extended_opcode = 6;
        let mod_ = 0b01;
        let modrm = self.calc_modrm(
            mod_,
            extended_opcode,
            self.get_reg_value(&self.register).unwrap(),
        );

        // p 1633
        Ok(vec![
            IntermediateCode::Byte(opcode),
            IntermediateCode::Byte(modrm),
            IntermediateCode::Byte(self.offset.value.parse::<i8>()? as u8),
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
            vec![
                vec![TokenType::Pop].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
            ],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        // p 1633
        let opcode = 0x58 | self.get_reg_value(&self.operand).unwrap();
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
                vec![TokenType::Compare].into_iter().collect::<HashSet<_>>(),
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
                vec![TokenType::Register]
                    .into_iter()
                    .collect::<HashSet<_>>(),
            ],
            vec![&self.operation, &self.left_operand, &self.right_operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        // p 725
        // Contrary to convention the order of these operands is more
        // in line with what you would expect. With x < y:
        // cmp x, y
        // will make jle jump.
        //
        // In a conventional assembly language with AT&T syntax this
        // would make jge jump, because there the second argument is
        // compared to the first.
        let opcode = 0x39;
        let modrm = self.calc_modrm(
            0b11,
            self.get_reg_value(&self.right_operand).unwrap(),
            self.get_reg_value(&self.left_operand).unwrap(),
        );

        Ok(vec![
            IntermediateCode::Byte(opcode),
            IntermediateCode::Byte(modrm),
        ])
    }
}

struct InstructionJumpIf<'a> {
    operation: &'a Token,
    operand: &'a Token,
}

impl<'a> Instruction for InstructionJumpIf<'a> {
    fn validate(&self) -> Result<(), Box<error::Error>> {
        self.validate_tokens(
            vec![
                vec![
                    TokenType::JumpIfEqual,
                    TokenType::JumpIfNotEqual,
                    TokenType::JumpIfLess,
                    TokenType::JumpIfLessEqual,
                    TokenType::JumpIfGreater,
                    TokenType::JumpIfGreaterEqual,
                ]
                .into_iter()
                .collect::<HashSet<_>>(),
                vec![TokenType::LabelReference]
                    .into_iter()
                    .collect::<HashSet<_>>(),
            ],
            vec![&self.operation, &self.operand],
        )
    }

    fn compile(&self) -> Result<Vec<IntermediateCode>, Box<error::Error>> {
        self.validate()?;

        // p 1058
        // Only supports near (32 bit) jumps
        let opcode1 = 0x0f;
        let opcode2 = match self.operation.t {
            Some(TokenType::JumpIfEqual) => 0x84,
            Some(TokenType::JumpIfNotEqual) => 0x85,
            Some(TokenType::JumpIfLess) => 0x8c,
            Some(TokenType::JumpIfLessEqual) => 0x8e,
            Some(TokenType::JumpIfGreater) => 0x8f,
            Some(TokenType::JumpIfGreaterEqual) => 0x8d,
            _ => panic!(
                "Attempting to compile invalid InstructionJumpIf: {:?}.",
                self.operation.t
            ),
        };
        Ok(vec![
            IntermediateCode::Byte(opcode1),
            IntermediateCode::Byte(opcode2),
            IntermediateCode::Displacement32(self.operand.value.clone()),
        ])
    }
}

#[cfg(test)]
mod test_instructions {
    use super::*;

    #[test]
    #[should_panic(expected = "mod_ should be 2 bits but is 0b111")]
    fn test_calc_modrm_panic() {
        let i = InstructionJump {
            operation: &Token {
                t: None,
                value: "".to_string(),
            },

            operand: &Token {
                t: None,
                value: "".to_string(),
            },
        };

        i.calc_modrm(0b111, 0, 0);
    }

    #[test]
    fn test_calc_modrm() {
        let i = InstructionJump {
            operation: &Token {
                t: None,
                value: "".to_string(),
            },

            operand: &Token {
                t: None,
                value: "".to_string(),
            },
        };

        assert_eq!(i.calc_modrm(0b11, 0b011, 0b100), 0b11011100);
    }

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
                IntermediateCode::Byte(0xb8 | instruction.get_reg_value(&register).unwrap()),
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
                IntermediateCode::Byte(0xb8 | instruction.get_reg_value(&register).unwrap()),
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
                IntermediateCode::Byte(0xb8 | instruction.get_reg_value(&register).unwrap()),
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
                IntermediateCode::Byte(0b11000000 | instruction.get_reg_value(&register).unwrap()),
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
                IntermediateCode::Byte(0b11000000 | instruction.get_reg_value(&register).unwrap()),
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
    fn test_jump_if() {
        let operation = Token {
            t: Some(TokenType::JumpIfEqual),
            value: "🦘=".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::LabelReference),
            value: "test_label".to_string(),
        };
        let instruction = InstructionJumpIf {
            operation: &operation,
            operand: &operand,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0x0f),
                IntermediateCode::Byte(0x84),
                IntermediateCode::Displacement32("test_label".to_string())
            ],
            &bytes
        ));
    }

    #[test]
    fn test_push_immediate1() {
        let operation = Token {
            t: Some(TokenType::Push),
            value: "📥".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "174285409".to_string(),
        };
        let instruction = InstructionPushImmediate {
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
    fn test_push_modrm1() {
        let operation = Token {
            t: Some(TokenType::Push),
            value: "📥".to_string(),
        };
        let register = Token {
            t: Some(TokenType::Register),
            value: "⬇".to_string(),
        };
        let offset = Token {
            t: Some(TokenType::Value),
            value: "-4".to_string(),
        };
        let instruction = InstructionPushModRM {
            operation: &operation,
            register: &register,
            offset: &offset,
        };

        let bytes = instruction.compile().unwrap();
        assert!(vec_compare(
            &[
                IntermediateCode::Byte(0xff),
                IntermediateCode::Byte(0x75),
                IntermediateCode::Byte(0xfc),
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
        assert!(vec_compare(
            &[IntermediateCode::Byte(0x39), IntermediateCode::Byte(0xc3)],
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
    fn test_push_immediate_validate() {
        let operation = Token {
            t: Some(TokenType::Push),
            value: "📥".to_string(),
        };
        let operand = Token {
            t: Some(TokenType::Value),
            value: "123".to_string(),
        };
        let instruction = InstructionPushImmediate {
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
            Some(TokenType::Push) => Some(Box::new(InstructionPushImmediate {
                operation: &tokens[0],
                operand: &tokens[1],
            })),
            Some(TokenType::Pop) => Some(Box::new(InstructionPop {
                operation: &tokens[0],
                operand: &tokens[1],
            })),
            Some(TokenType::Compare) => Some(Box::new(InstructionCompare {
                operation: &tokens[0],
                left_operand: &tokens[1],
                right_operand: &tokens[2],
            })),
            Some(TokenType::JumpIfEqual)
            | Some(TokenType::JumpIfNotEqual)
            | Some(TokenType::JumpIfLess)
            | Some(TokenType::JumpIfLessEqual)
            | Some(TokenType::JumpIfGreater)
            | Some(TokenType::JumpIfGreaterEqual) => Some(Box::new(InstructionJumpIf {
                operation: &tokens[0],
                operand: &tokens[1],
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
