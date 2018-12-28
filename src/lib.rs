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
mod tokenizer;

use common::{serialize_le, serialize_signed_le, IntermediateCode, TokenType};
use compiler::*;
use config::*;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::{error, fs};
use tokenizer::*;

const VIRTUAL_ENTRY_POINT: u32 = 0x08049000;

fn process(filename: &str) -> Result<Vec<u8>, Box<error::Error>> {
    let content = fs::read_to_string(filename)?;

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

    // This maps the index of a displacement in the intermediate
    // program to an offset so that:
    // displacement index - offset = index where instruction ends
    // This is done because RIP addressing is relative to the *end* of
    // the current instruction.
    let mut intermediate_index_instruction_offset = HashMap::new();

    for line in content.split('\n') {
        let tokens = tokenize(line)?;
        // line was a comment
        if tokens.is_empty() {
            continue;
        }

        if tokens.len() == 1 && tokens[0].clone().t.unwrap() == TokenType::Label {
            // labels should point to the next instruction
            labels.insert(tokens[0].value.clone(), intermediate_program.len());
        } else {
            let mut intermediate_instruction = compile(tokens)?;
            let mut padded_intermediate_instruction = vec![];
            let mut displacements = vec![];
            for (i, intermediate) in intermediate_instruction.iter().enumerate() {
                padded_intermediate_instruction.push(intermediate.clone());
                match intermediate {
                    IntermediateCode::Displacement32(_) => {
                        displacements.push(padded_intermediate_instruction.len() - 1);
                        padded_intermediate_instruction
                            .append(&mut vec![IntermediateCode::Padding; 3]);
                    }
                    _ => {}
                }
            }

            for displacement in displacements {
                // println!(
                //     "from displacement {} to end is {} - {} = {}",
                //     displacement,
                //     padded_intermediate_instruction.len(),
                //     displacement,
                //     padded_intermediate_instruction.len() - displacement
                // );
                intermediate_index_instruction_offset.insert(
                    intermediate_program.len() + displacement,
                    padded_intermediate_instruction.len() - displacement,
                );
            }

            intermediate_program.append(&mut padded_intermediate_instruction);
        }
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
                        i as i32 + *intermediate_index_instruction_offset.get(&i).unwrap() as i32;
                    let displacement = *target_i as i32 - instruction_end;
                    // println!(
                    //     "instruction end is {}\ntarget is {} at {}\ndisplacement is {}\n",
                    //     instruction_end, s, target_i, displacement
                    // );
                    serialize_signed_le(displacement)
                }
                None => panic!("Unknown label {}", s),
            },
            IntermediateCode::Padding => vec![],
        };
        program.append(&mut bytes);
    }

    Ok(program)
}

fn create_program_header(program_size: u32) -> Vec<u8> {
    let mut program_header: Vec<u8> = vec![];
    // all members are 4 bytes
    // typedef struct elf32_phdr{
    //     Elf32_Word	p_type;
    //     Elf32_Off	p_offset;
    //     Elf32_Addr	p_vaddr;
    //     Elf32_Addr	p_paddr;
    //     Elf32_Word	p_filesz;
    //     Elf32_Word	p_memsz;
    //     Elf32_Word	p_flags;
    //     Elf32_Word	p_align;
    // } Elf32_Phdr;

    // For now just create one program header entry. It will point to
    // the entry point.

    // p_type
    const PT_LOAD: u32 = 1;
    program_header.append(&mut serialize_le(PT_LOAD));

    // p_offset
    const PHYSICAL_ENTRY_POINT: u32 = 0x1000; // align on page size
    program_header.append(&mut serialize_le(PHYSICAL_ENTRY_POINT));

    // p_vaddr
    program_header.append(&mut serialize_le(VIRTUAL_ENTRY_POINT));

    // p_paddr (unspecified on System V, but seems to usually be virtual entry point)
    program_header.append(&mut serialize_le(VIRTUAL_ENTRY_POINT));

    // p_filesz
    program_header.append(&mut serialize_le(program_size));

    // p_memsz
    program_header.append(&mut serialize_le(program_size));

    // p_flags
    const PF_X_R: u32 = 1 | (1 << 2);
    program_header.append(&mut serialize_le(PF_X_R));

    // p_align
    // align on 4KB
    program_header.append(&mut serialize_le(0x1000));

    program_header
}

fn create_elf_header() -> Vec<u8> {
    let mut header: Vec<u8> = vec![];

    // Magic number
    header.append(&mut vec![0x7f, 0x45, 0x4c, 0x46]);

    // 32 bit
    header.push(0x01);

    // little endian
    header.push(0x01);

    // ELF version 1
    header.push(0x01);

    // Target operation system ABI (System V)
    header.push(0x00);

    // ABI version (currently unused)
    header.push(0x00);

    // EIPAD (currently unused)
    header.append(&mut vec![0x00; 7]);

    // Object file type (ET_EXEC)
    header.append(&mut vec![0x02, 0x00]);

    // Target architecture x86
    header.append(&mut vec![0x03, 0x00]);

    // ELF version 1
    header.append(&mut serialize_le(1));

    // e_entry
    header.append(&mut serialize_le(VIRTUAL_ENTRY_POINT));

    // Start of program header table (immediately after this header)
    header.append(&mut serialize_le(0x34));

    // Start of section header table
    header.append(&mut serialize_le(0x00));

    // eflags
    header.append(&mut vec![0x00; 4]);

    // Size of this header
    header.append(&mut vec![52, 0x00]);

    // e_phentsize: size of a program header table entry
    header.append(&mut vec![32, 0x00]);

    // e_phnum: number of entries in program header table
    header.append(&mut vec![0x01, 0x00]);

    // TODO: Size of a section header table entry
    header.append(&mut vec![0x00, 0x00]);

    // TODO: Number of entries in section header table
    header.append(&mut vec![0x00, 0x00]);

    // TODO: index of section header table entry that contains section names
    header.append(&mut vec![0x00, 0x00]);

    header
}

#[cfg(test)]
mod test_elf {
    use super::*;

    #[test]
    fn test_elf_header_length() {
        assert_eq!(create_elf_header().len(), 52);
    }

    #[test]
    fn test_program_header_length() {
        assert_eq!(create_program_header(0).len(), 8 * 4);
    }
}

pub fn run(config: Config) -> std::io::Result<()> {
    println!("compile {}", config.filename);

    let elf_header = create_elf_header();
    let program = process(&config.filename).unwrap();
    let program_header = create_program_header(program.len() as u32);

    let mut file = fs::File::create("a.out")?;
    file.set_permissions(PermissionsExt::from_mode(0o755))?;

    file.write_all(&elf_header)?;
    file.write_all(&program_header)?;

    // pad 4KB page
    let padding = vec![0; 0x1000 - elf_header.len() - program_header.len()];
    file.write_all(&padding)?;

    file.write_all(&program)?;

    Ok(())
}
