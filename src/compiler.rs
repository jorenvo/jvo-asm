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

pub fn compile(tokens: Vec<Token>) -> Result<Vec<u8>, Box<error::Error>> {
    Err(Box::new(CompileError {
        msg: "Not implemented!".to_string(),
    }))
}
