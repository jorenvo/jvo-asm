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

pub fn run(config: Config) {
    println!("compile {}", config.filename);

    let _program = process(&config.filename).unwrap();
}
