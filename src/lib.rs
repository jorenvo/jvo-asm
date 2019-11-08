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
mod common;
mod compiler;
pub mod config;
mod executable;
mod tokenizer;

use common::*;
use compiler::*;
use config::*;
use executable::{Executable, ELF};
use std::collections::HashMap;
use std::{error, fs};
use tokenizer::*;

fn process(filename: &str) -> Result<Vec<DataSection>, Box<dyn error::Error>> {
    let content = fs::read_to_string(filename)?;

    // Contains a section for the executable code and other data
    // sections. The executable code will have CODE_SECTION_NAME as
    // it's key.
    let mut sections: Vec<DataSection> = vec![];

    // The intermediate program consists of IntermediateCode. The
    // instructions are responsible for compiling
    // IntermediateCode. The intermediate program is padded so that
    // displacements take up the right amount of space (e.g. 1
    // Displacement32 + 3 Padding). This way the index in the
    // intermediate program vector can be used for offset
    // calculations. Note that you cannot use the compiled program for
    // this. You do not know addresses of instructions not yet
    // compiled if we didn't do this padding.
    let mut intermediate_program: Vec<IntermediateCode> = vec![];

    // This maps a label String to the index in the intermediate
    // program it points to.
    let mut labels = HashMap::new();

    // This holds the size of all processed data sections.
    let mut data_section_size: usize = 0;

    // This maps constant names to the tokens they should be replaced
    // with.
    let mut constants = HashMap::new();

    // This maps the index of a displacement in the intermediate
    // program to an offset so that:
    // displacement index - offset = index where instruction ends
    // This is done because RIP addressing is relative to the *end* of
    // the current instruction.
    let mut intermediate_index_instruction_offset = HashMap::new();

    for line in content.split('\n') {
        let mut tokens = tokenize(line)?;
        // Line was a comment.
        if tokens.is_empty() {
            continue;
        }

        // These tokens will not be translated to bytes in the
        // executable.
        match tokens[0].t {
            Some(TokenType::Constant) => {
                constants.insert(tokens[0].value.clone(), tokens[1].clone());
                continue;
            }
            Some(TokenType::Label) => {
                // Labels should point to the next instruction.
                labels.insert(tokens[0].value.clone(), intermediate_program.len());
                continue;
            }
            Some(TokenType::Section) => {
                // Sections will be referenced with Constants
                // afterwards. Create a Value Token with the virtual
                // address these constants will be replaced by.
                let virtual_address = Token {
                    t: Some(TokenType::Value),
                    value: (DATA_SECTION_VIRTUAL_START as usize + data_section_size).to_string(),
                };
                let section_name = &tokens[0].value;
                constants.insert(section_name.clone(), virtual_address);
                data_section_size += PAGE_SIZE as usize; // TODO data sections are assumed to be 4KB

                let mut section_data = vec![];
                for token in &tokens[1..] {
                    match token.t {
                        // In data sections 32 bit values are tokenized as
                        // Memory (no preceding $).
                        Some(TokenType::Memory) => section_data
                            .extend_from_slice(&token.value.parse::<i32>()?.to_le_bytes()),
                        _ => panic!("Unsupported token in data section: {:?}", token),
                    }
                }

                sections.push(DataSection {
                    name: section_name.clone(),
                    bytes: section_data,
                });

                continue;
            }
            _ => {}
        };

        // Replace ConstantReferences.
        tokens = tokens
            .into_iter()
            .map(|token| match token.t {
                Some(TokenType::ConstantReference) => match constants.get(&token.value) {
                    Some(token) => token.clone(),
                    _ => panic!("ConstantReference {} not found", token.value),
                },
                _ => token,
            })
            .collect();

        let intermediate_instruction = compile(tokens)?;
        let mut padded_intermediate_instruction = vec![];
        let mut displacements = vec![];
        for intermediate in intermediate_instruction {
            padded_intermediate_instruction.push(intermediate.clone());

            if let IntermediateCode::Displacement32(_) = intermediate {
                displacements.push(padded_intermediate_instruction.len() - 1);
                padded_intermediate_instruction.append(&mut vec![IntermediateCode::Padding; 3]);
            }
        }

        for displacement in displacements {
            intermediate_index_instruction_offset.insert(
                intermediate_program.len() + displacement,
                padded_intermediate_instruction.len() - displacement,
            );
        }

        intermediate_program.append(&mut padded_intermediate_instruction);
    }

    // This contains the compiled program. It is the intermediate
    // program with all the intermediate symbols translated to bytes.
    let mut program: Vec<u8> = vec![];
    for (i, intermediate) in intermediate_program.iter().enumerate() {
        let mut bytes = match intermediate {
            IntermediateCode::Byte(b) => vec![*b],
            IntermediateCode::Displacement32(s) => match labels.get(s) {
                Some(target_i) => {
                    let instruction_end =
                        i as i32 + intermediate_index_instruction_offset[&i] as i32;
                    let displacement = *target_i as i32 - instruction_end;
                    let mut v = Vec::new();
                    v.extend_from_slice(&displacement.to_le_bytes());
                    v
                }
                None => panic!("Unknown label {}", s),
            },
            IntermediateCode::Padding => vec![],
        };
        program.append(&mut bytes);
    }

    sections.push(DataSection {
        name: CODE_SECTION_NAME.to_string(),
        bytes: program,
    });
    Ok(sections)
}

pub fn run(config: Config) -> std::io::Result<()> {
    println!("compile {}", config.filename);

    let data_sections = process(&config.filename).unwrap();

    // branch here
    let mut elf: ELF = ELF {};
    let file = fs::File::create("a.out")?;
    elf.create(data_sections, file)?;

    Ok(())
}
