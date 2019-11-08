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
use common::ExecutableFormat;

pub struct Config {
    pub filename: String,
    pub exec_format: ExecutableFormat,
}

impl Config {
    pub fn new(mut args: Vec<String>) -> Result<Config, String> {
        let program_name = args.remove(0);

        if args.is_empty() {
            Err(format!("Usage: {} program.jas", program_name))
        } else {
            Ok(Config {
                filename: args.remove(0),
                exec_format: ExecutableFormat::ELF,
            })
        }
    }
}
