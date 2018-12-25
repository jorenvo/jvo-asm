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

use compiler::*;
use config::*;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::{error, fs};
use tokenizer::*;

const VIRTUAL_ENTRY_POINT: u32 = 0x08049000;

fn process(filename: &str) -> Result<Vec<u8>, Box<error::Error>> {
    let mut program: Vec<u8> = vec![];
    let content = fs::read_to_string(filename)?;

    for line in content.split('\n') {
        let tokens = tokenize(line)?;
        // line was a comment
        if tokens.is_empty() {
            continue;
        }

        let mut bytes = compile(tokens)?;

        program.append(&mut bytes);
    }

    Ok(program)
}

fn serialize(n: u32) -> Vec<u8> {
    // this serializes to a little endian byte array
    vec![
        (n & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        ((n >> 16) & 0xff) as u8,
        (n >> 24) as u8,
    ]
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
    program_header.append(&mut serialize(PT_LOAD));

    // p_offset
    const PHYSICAL_ENTRY_POINT: u32 = 0x1000; // align on page size
    program_header.append(&mut serialize(PHYSICAL_ENTRY_POINT));

    // p_vaddr
    program_header.append(&mut serialize(VIRTUAL_ENTRY_POINT));

    // p_paddr (unspecified on System V, but seems to usually be virtual entry point)
    program_header.append(&mut serialize(VIRTUAL_ENTRY_POINT));

    // p_filesz
    program_header.append(&mut serialize(program_size));

    // p_memsz
    program_header.append(&mut serialize(program_size));

    // p_flags
    const PF_X_R: u32 = 1 | (1 << 2);
    program_header.append(&mut serialize(PF_X_R));

    // p_align
    // align on 4KB
    program_header.append(&mut serialize(0x1000));

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
    header.append(&mut serialize(1));

    // e_entry
    header.append(&mut serialize(VIRTUAL_ENTRY_POINT));

    // Start of program header table (immediately after this header)
    header.append(&mut serialize(0x34));

    // Start of section header table
    header.append(&mut serialize(0x00));

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

    #[test]
    fn test_serialize() {
        let v = 0x08049000;
        let serialized = serialize(v);

        assert_eq!(serialized[3], 0x08);
        assert_eq!(serialized[2], 0x04);
        assert_eq!(serialized[1], 0x90);
        assert_eq!(serialized[0], 0x00);
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
