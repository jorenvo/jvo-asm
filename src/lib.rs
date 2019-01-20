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

use common::{IntermediateCode, Token, TokenType};
use compiler::*;
use config::*;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::{error, fs};
use tokenizer::*;

const CODE_SECTION_NAME: &str = ".code";
const STRTAB_SECTION_NAME: &str = ".shstrtab";

const PHYSICAL_ENTRY_POINT: u32 = 0x1000; // align on page size
const STRTABLE_PHYSICAL_ENTRY_POINT: u32 = 0x400;
const DATA_SECTION_START: u32 = 0x600;

const VIRTUAL_ENTRY_POINT: u32 = 0x08049000;
const DATA_SECTION_VIRTUAL_START: u32 =
    VIRTUAL_ENTRY_POINT - PHYSICAL_ENTRY_POINT - DATA_SECTION_START;

fn process(filename: &str) -> Result<BTreeMap<String, Vec<u8>>, Box<error::Error>> {
    let content = fs::read_to_string(filename)?;

    // Maps section name to code. The executable code will have
    // CODE_SECTION_NAME as it's key. This uses a BTreeMap because
    // it's ordered.
    let mut section_name_to_code = BTreeMap::new();

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
                data_section_size += tokens[1..].len();

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

                section_name_to_code.insert(section_name.clone(), section_data);

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

        let mut intermediate_instruction = compile(tokens)?;
        let mut padded_intermediate_instruction = vec![];
        let mut displacements = vec![];
        for intermediate in intermediate_instruction {
            padded_intermediate_instruction.push(intermediate.clone());
            match intermediate {
                IntermediateCode::Displacement32(_) => {
                    displacements.push(padded_intermediate_instruction.len() - 1);
                    padded_intermediate_instruction.append(&mut vec![IntermediateCode::Padding; 3]);
                }
                _ => {}
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
                        i as i32 + *intermediate_index_instruction_offset.get(&i).unwrap() as i32;
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

    section_name_to_code.insert(CODE_SECTION_NAME.to_string(), program);
    Ok(section_name_to_code)
}

fn create_string_table(strings: Vec<&String>) -> Vec<u8> {
    let mut table: Vec<u8> = vec![0x00]; // first byte is defined to be null
    for s in strings {
        table.extend(s.bytes());
        table.push(0x00);
    }

    table
}

fn create_section_header_entry(
    sh_name: u32,
    sh_type: u32,
    sh_flags: u32,
    sh_addr: u32,
    sh_offset: u32,
    sh_size: u32,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u32,
    sh_entsize: u32,
) -> Vec<u8> {
    let mut entry: Vec<u8> = vec![];
    // typedef struct
    // {
    //     Elf32_Word    sh_name;                /* Section name (string tbl index) */
    //     Elf32_Word    sh_type;                /* Section type */
    //     Elf32_Word    sh_flags;               /* Section flags */
    //     Elf32_Addr    sh_addr;                /* Section virtual addr at execution */
    //     Elf32_Off     sh_offset;              /* Section file offset */
    //     Elf32_Word    sh_size;                /* Section size in bytes */
    //     Elf32_Word    sh_link;                /* Link to another section */
    //     Elf32_Word    sh_info;                /* Additional section information */
    //     Elf32_Word    sh_addralign;           /* Section alignment */
    //     Elf32_Word    sh_entsize;             /* Entry size if section holds table */
    // } Elf32_Shdr;

    // sh_name
    entry.extend_from_slice(&sh_name.to_le_bytes());

    // sh_type
    entry.extend_from_slice(&sh_type.to_le_bytes());

    // sh_flags
    entry.extend_from_slice(&sh_flags.to_le_bytes());

    // sh_addr
    entry.extend_from_slice(&sh_addr.to_le_bytes());

    // sh_offset
    entry.extend_from_slice(&sh_offset.to_le_bytes());

    // sh_size
    entry.extend_from_slice(&sh_size.to_le_bytes());

    // sh_link
    entry.extend_from_slice(&sh_link.to_le_bytes());

    // sh_info
    entry.extend_from_slice(&sh_info.to_le_bytes());

    // sh_addralign
    entry.extend_from_slice(&sh_addralign.to_le_bytes());

    // sh_entsize
    entry.extend_from_slice(&sh_entsize.to_le_bytes());

    entry
}

fn create_section_header(
    program_size: u32,
    data_section_sizes: Vec<u32>,
    data_section_names: Vec<&String>,
    strtable_size: u32,
) -> Vec<u8> {
    const SHT_NULL: u32 = 0x00;
    const SHT_PROGBITS: u32 = 0x01;
    const SHT_STRTAB: u32 = 0x03;
    const SHF_WRITE: u32 = 0x01;
    const SHF_ALLOC: u32 = 0x02;
    const SHF_EXECINSTR: u32 = 0x04;

    let mut section_header: Vec<u8> = vec![];
    let mut strtab_index = 0x01;

    // sentinel
    section_header.append(&mut create_section_header_entry(
        0x00, SHT_NULL, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ));

    let mut virtual_address = DATA_SECTION_VIRTUAL_START;
    let mut section_start = DATA_SECTION_START;
    for (index, size) in data_section_sizes.iter().enumerate() {
        if section_start >= PHYSICAL_ENTRY_POINT {
            // TODO: the physical entry point could be incremented by
            // 4 KiB here instead of panicking.
            panic!("Data section too big.");
        }

        section_header.append(&mut create_section_header_entry(
            strtab_index,
            SHT_PROGBITS,
            SHF_WRITE | SHF_ALLOC,
            virtual_address,
            section_start,
            *size,
            0x00,
            0x00,
            0x01, // (no alignment constraint)
            0x00,
        ));

        section_start += size;
        virtual_address += size;
        strtab_index += data_section_names[index].len() as u32 + 1;
    }

    // executable code
    section_header.append(&mut create_section_header_entry(
        strtab_index,
        SHT_PROGBITS,
        SHF_ALLOC | SHF_EXECINSTR,
        VIRTUAL_ENTRY_POINT,
        PHYSICAL_ENTRY_POINT,
        program_size,
        0x00,
        0x00,
        0x01, // (no alignment constraint)
        0x00,
    ));

    // string table
    section_header.append(&mut create_section_header_entry(
        strtab_index + (CODE_SECTION_NAME.len() + 1) as u32,
        SHT_STRTAB,
        0x00,
        0x00,
        STRTABLE_PHYSICAL_ENTRY_POINT,
        strtable_size,
        0x00,
        0x00,
        0x01, // (no alignment constraint)
        0x00,
    ));

    section_header
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
    program_header.extend_from_slice(&PT_LOAD.to_le_bytes());

    // p_offset
    program_header.extend_from_slice(&PHYSICAL_ENTRY_POINT.to_le_bytes());

    // p_vaddr
    program_header.extend_from_slice(&VIRTUAL_ENTRY_POINT.to_le_bytes());

    // p_paddr (unspecified on System V, but seems to usually be virtual entry point)
    program_header.extend_from_slice(&VIRTUAL_ENTRY_POINT.to_le_bytes());

    // p_filesz
    program_header.extend_from_slice(&program_size.to_le_bytes());

    // p_memsz
    program_header.extend_from_slice(&program_size.to_le_bytes());

    // p_flags
    const PF_X_R: u32 = 1 | (1 << 2);
    program_header.extend_from_slice(&PF_X_R.to_le_bytes());

    // p_align
    // align on 4KB
    program_header.extend_from_slice(&(0x1000 as u32).to_le_bytes());

    program_header
}

fn create_elf_header(number_of_sections: u32) -> Vec<u8> {
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
    header.extend_from_slice(&(1 as u32).to_le_bytes());

    // e_entry
    header.extend_from_slice(&VIRTUAL_ENTRY_POINT.to_le_bytes());

    // Start of program header table (immediately after this header)
    header.extend_from_slice(&(0x34 as u32).to_le_bytes());

    // e_shoff: Start of section header table
    header.extend_from_slice(&(0x54 as u32).to_le_bytes());

    // eflags
    header.append(&mut vec![0x00; 4]);

    // Size of this header
    header.append(&mut vec![52, 0x00]);

    // e_phentsize: size of a program header table entry
    header.append(&mut vec![32, 0x00]);

    // e_phnum: number of entries in program header table
    header.append(&mut vec![0x01, 0x00]);

    // e_shentsize: size of a section header table entry
    header.append(&mut vec![40, 0x00]);

    // e_shnum: number of entries in section header table
    header.append(&mut vec![number_of_sections as u8, 0x00]);

    // e_shstrndx: index of section header table entry that contains section names
    header.append(&mut vec![(number_of_sections - 1) as u8, 0x00]);

    header
}

#[cfg(test)]
mod test_elf {
    use super::*;

    #[test]
    fn test_elf_header_length() {
        assert_eq!(create_elf_header(1).len(), 52);
    }

    #[test]
    fn test_section_header_length() {
        const BYTES_PER_FIELD: usize = 4;
        const FIELDS_PER_ENTRY: usize = 10;
        const ENTRIES: usize = 3;

        assert_eq!(
            create_section_header(0, vec![], vec![], 0).len(),
            BYTES_PER_FIELD * FIELDS_PER_ENTRY * ENTRIES
        );
    }

    #[test]
    fn test_program_header_length() {
        assert_eq!(create_program_header(0).len(), 8 * 4);
    }
}

pub fn run(config: Config) -> std::io::Result<()> {
    println!("compile {}", config.filename);

    let mut data_sections = process(&config.filename).unwrap();

    // + 2 for string table and null sentinel
    let elf_header = create_elf_header(data_sections.len() as u32 + 2);
    let program = data_sections.remove(CODE_SECTION_NAME).unwrap();
    let program_header = create_program_header(program.len() as u32);

    let mut string_table = create_string_table(data_sections.keys().collect());

    // add str name for code and strtab at end of table
    string_table.extend(CODE_SECTION_NAME.bytes());
    string_table.push(0x00);
    string_table.extend(STRTAB_SECTION_NAME.bytes());
    string_table.push(0x00);

    let data_section_sizes = data_sections.iter().map(|(_, v)| v.len() as u32).collect();
    let section_header = create_section_header(
        program.len() as u32,
        data_section_sizes,
        data_sections.keys().collect(),
        string_table.len() as u32,
    );
    let mut file = fs::File::create("a.out")?;
    file.set_permissions(PermissionsExt::from_mode(0o755))?;

    file.write_all(&elf_header)?;
    file.write_all(&program_header)?;
    file.write_all(&section_header)?;

    // string table starts at STRTABLE_PHYSICAL_ENTRY_POINT
    let padding = vec![
        0;
        STRTABLE_PHYSICAL_ENTRY_POINT as usize
            - elf_header.len()
            - program_header.len()
            - section_header.len()
    ];
    file.write_all(&padding)?;
    file.write_all(&string_table)?;

    // insert data sections
    // DATA_SECTION_START
    let padding = vec![
        0;
        DATA_SECTION_START as usize
            - STRTABLE_PHYSICAL_ENTRY_POINT as usize
            - string_table.len()
    ];
    file.write_all(&padding)?;
    for (_, data) in data_sections.iter() {
        file.write_all(&data)?;
    }

    // pad 4KB page
    let padding = vec![
        0;
        PHYSICAL_ENTRY_POINT as usize
            - DATA_SECTION_START as usize
            - data_sections
                .iter()
                .map(|(_, bytes)| bytes.len())
                .sum::<usize>()
    ];
    file.write_all(&padding)?;
    file.write_all(&program)?;

    Ok(())
}
