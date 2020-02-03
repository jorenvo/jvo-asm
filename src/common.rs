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
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TokenType {
    Move,
    MoveQuad,
    Add,
    Subtract,
    Multiply,
    JumpIfEqual,
    JumpIfNotEqual,
    JumpIfLess,
    JumpIfLessEqual,
    JumpIfGreater,
    JumpIfGreaterEqual,
    Jump,
    Call,
    Push,
    Pop,
    Value,
    Memory,
    Register,
    Return,
    Compare,
    Interrupt,
    Syscall,
    Label,
    LabelReference,
    Constant,
    ConstantReference, // <- this should be translated before compilation
    Section,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub t: Option<TokenType>,
    pub value: String,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IntermediateCode {
    Byte(u8),
    Displacement32(String),

    Padding,
}

pub struct DataSection {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(PartialEq)]
pub enum ExecutableFormat {
    MachO,
    ELF,
}

pub const DATA_SECTION_PHYSICAL_START: u32 = 0x1000;
pub const CODE_SECTION_NAME: &str = ".code";
pub const DATA_SECTION_VIRTUAL_START_32: u32 = 0x0804_9000;
pub const DATA_SECTION_VIRTUAL_START_64: u64 = 0x0000_0001_0000_0000;
pub const PAGE_SIZE: u32 = 0x1000;
