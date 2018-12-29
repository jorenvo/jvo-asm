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
#![allow(unused)]
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
    Move,
    Add,
    Jump,
    Value,
    Memory,
    BasePointerMemory,
    Register,
    Return,
    Interrupt,
    Label,
    LabelReference,
    Constant,
    ConstantReference, // <- this should be translated before compilation
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

// TODO: replace all serialize_* functions with native
// int_to_from_bytes when 1.32.0 is released.
// https://github.com/rust-lang/rust/commit/68a26ec647147d70bcd7f0e7f56a0bf9fedb5f06
pub fn serialize(n: u32) -> Vec<u8> {
    // this serializes to a little endian byte array
    vec![
        (n >> 24) as u8,
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ]
}

pub fn serialize_le(n: u32) -> Vec<u8> {
    let mut b = serialize(n);
    b.reverse();
    b
}

pub fn serialize_signed(n: i32) -> Vec<u8> {
    // this serializes to a little endian byte array
    vec![
        (n >> 24) as u8,
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ]
}

pub fn serialize_signed_le(n: i32) -> Vec<u8> {
    let mut b = serialize_signed(n);
    b.reverse();
    b
}

#[cfg(test)]
mod test_common {
    use super::*;

    #[test]
    fn test_serialize() {
        let v = 0x08049000;
        let serialized = serialize(v);

        assert_eq!(serialized[0], 0x08);
        assert_eq!(serialized[1], 0x04);
        assert_eq!(serialized[2], 0x90);
        assert_eq!(serialized[3], 0x00);
    }

    #[test]
    fn test_serialize_little_endian() {
        let v = 0x08049000;
        let serialized = serialize_le(v);

        assert_eq!(serialized[3], 0x08);
        assert_eq!(serialized[2], 0x04);
        assert_eq!(serialized[1], 0x90);
        assert_eq!(serialized[0], 0x00);
    }
}
