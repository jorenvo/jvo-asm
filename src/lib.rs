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
use std::{error, fs};
use tokenizer::*;

fn process(filename: &str) -> Result<Vec<u8>, Box<error::Error>> {
    let mut program: Vec<u8> = vec![];
    let content = fs::read_to_string(filename)?;

    for line in content.split('\n') {
        let tokens = tokenize(line)?;
        let mut bytes = compile(tokens)?;

        program.append(&mut bytes);
    }

    Ok(program)
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
    header.append(&mut vec![0x00, 0x02]);

    // Target architecture x86
    header.append(&mut vec![0x00, 0x03]);

    // ELF version 1
    header.append(&mut vec![0x00, 0x00, 0x00, 0x01]);

    // TODO: entry point
    header.append(&mut vec![0x00; 4]);

    // Start of program header table (immediately after this header)
    header.append(&mut vec![0x00, 0x00, 0x00, 0x34]);

    // TODO: Start of section header table
    header.append(&mut vec![0x00, 0x00, 0x00, 0x00]);

    // TODO: eflags
    header.append(&mut vec![0x00, 0x00, 0x00, 0x00]);

    // Size of this header
    header.append(&mut vec![0x00, 52]);

    // TODO: Size of a program header table entry
    header.append(&mut vec![0x00, 0x00]);

    // TODO: Number of entries in program header table
    header.append(&mut vec![0x00, 0x00]);

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
}

pub fn run(config: Config) {
    println!("compile {}", config.filename);

    let _header = create_elf_header();
    let _program = process(&config.filename).unwrap();
}
