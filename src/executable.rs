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
use std::fs;
use std::io::Write;

const STRTABLE_PHYSICAL_ENTRY_POINT: u32 = 0x400;
const STRTAB_SECTION_NAME: &str = ".shstrtab";

pub trait Executable {
    fn create(&mut self, data_sections: Vec<DataSection>, file: fs::File) -> std::io::Result<()>;
}

pub struct MachO {}

impl MachO {
    pub fn create_header(&mut self, ncmds: u32, sizeofcmds: u32) -> Vec<u8> {
        // sizeof header:
        // 32 bytes

        const MAGIC: u32 = 0xfeed_facf; // Mach-O Big Endian (64-bit)
        const CPU_ARCH_ABI64: u32 = 0x1000000;
        const CPU_TYPE_I386: u32 = 7;
        const CPU_TYPE: u32 = CPU_ARCH_ABI64 | CPU_TYPE_I386; // x86_64
        const CPU_SUBTYPE: u32 = 3; // ALL
        const FILETYPE: u32 = 2; // EXECUTE

        const NOUNDEFS: u32 = 0x1;
        const FLAGS: u32 = NOUNDEFS;

        let mut header: Vec<u8> = vec![];
        header.extend_from_slice(&MAGIC.to_le_bytes());
        header.extend_from_slice(&CPU_TYPE.to_le_bytes());
        header.extend_from_slice(&CPU_SUBTYPE.to_le_bytes());
        header.extend_from_slice(&FILETYPE.to_le_bytes());
        header.extend_from_slice(&ncmds.to_le_bytes());
        header.extend_from_slice(&sizeofcmds.to_le_bytes());
        header.extend_from_slice(&FLAGS.to_le_bytes());
        header.extend_from_slice(&(0x00 as u32).to_le_bytes());

        header
    }

    pub fn create_segment_command(
        &mut self,
        section_size: u32,
        segname: &str,
        vmaddr: u64,
        fileoff: u64,
        nsects: u32,
    ) -> Vec<u8> {
        // sizeof segment command:
        // 72 bytes

        const CMD: u32 = 0x19; // LC_SEGMENT_64
        let mut command: Vec<u8> = vec![];

        command.extend_from_slice(&CMD.to_le_bytes());

        const CMD_SIZE: u32 = 72;
        const SECTION_SIZE: u32 = 80; // size of 64 bit section
        command.extend_from_slice(&(CMD_SIZE + SECTION_SIZE * nsects).to_le_bytes()); // cmdsize

        command.extend_from_slice(format!("{:\0<16}", segname).as_bytes()); // segname, 16 bytes
        command.extend_from_slice(&vmaddr.to_le_bytes()); // vmaddr

        // pagezero is empty
        if section_size == 0 {
            command.extend_from_slice(&(0x100000000 as u64).to_le_bytes()); // vmsize, should be the same as filesize
        } else {
            command.extend_from_slice(&(section_size as u64).to_le_bytes()); // vmsize, should be the same as filesize
        }

        command.extend_from_slice(&fileoff.to_le_bytes()); // fileoff, offset in file to map
        command.extend_from_slice(&(section_size as u64).to_le_bytes()); // filesize

        // todo
        if section_size == 0 {
            command.extend_from_slice(&(0x00 as u32).to_le_bytes()); // maxprot
            command.extend_from_slice(&(0x00 as u32).to_le_bytes()); // initprot
        } else {
            const VM_PROT_READ: u32 = 0x01;
            const VM_PROT_EXECUTE: u32 = 0x04;

            command.extend_from_slice(&(VM_PROT_READ | VM_PROT_EXECUTE).to_le_bytes()); // todo maxprot
            command.extend_from_slice(&(VM_PROT_READ | VM_PROT_EXECUTE).to_le_bytes());
            // todo initprot
        }

        command.extend_from_slice(&nsects.to_le_bytes()); // nsects

        command.extend_from_slice(&(0x00 as u32).to_le_bytes()); // todo flags

        command
    }

    pub fn create_thread_command(&mut self, rip: u64) -> Vec<u8> {
        let mut command: Vec<u8> = vec![];
        const LC_UNIXTHREAD: u32 = 0x5;

        command.extend_from_slice(&LC_UNIXTHREAD.to_le_bytes()); // cmd

        const SIZE_THREAD_COMMAND: u32 = 4 * 4 + 21 * 8;
        command.extend_from_slice(&SIZE_THREAD_COMMAND.to_le_bytes()); // todo size

        const X86_THREAD_STATE64: u32 = 0x4;
        command.extend_from_slice(&X86_THREAD_STATE64.to_le_bytes()); // flavor

        const NUM_REGISTERS: u32 = 21;
        command.extend_from_slice(&(NUM_REGISTERS * 8 / 4).to_le_bytes()); // count

        // rax
        // rbx
        // rcx
        // rdx
        // rdi
        // rsi
        // rbp
        // rsp
        // r8
        // r9
        // r10
        // r11
        // r12
        // r13
        // r14
        // r15
        for _ in 0..16 {
            command.extend_from_slice(&(0x00 as u64).to_le_bytes());
        }

        // rip
        command.extend_from_slice(&rip.to_le_bytes());

        // rflags
        // cs
        // fs
        // gs
        for _ in 0..4 {
            command.extend_from_slice(&(0x00 as u64).to_le_bytes());
        }

        command
    }

    pub fn create_section(
        &mut self,
        sectname: &str,
        segname: &str,
        vmaddr: u64,
        size: u64,
        fileoff: u32,
    ) -> Vec<u8> {
        // sizeof section:
        // 80 bytes
        let mut section: Vec<u8> = vec![];

        section.extend_from_slice(format!("{:\0<16}", sectname).as_bytes()); // sectname, 16 bytes
        section.extend_from_slice(format!("{:\0<16}", segname).as_bytes()); // segname, 16 bytes
        section.extend_from_slice(&vmaddr.to_le_bytes());
        section.extend_from_slice(&size.to_le_bytes());
        section.extend_from_slice(&fileoff.to_le_bytes());
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // todo align (2^3, so byte-aligned)
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // todo reloff
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // todo nreloc

        const S_ATTR_PURE_INSTRUCTIONS: u32 = 0x80000000;
        const S_ATTR_SOME_INSTRUCTIONS: u32 = 0x00000400;
        const INSTRUCTIONS_FLAG: u32 = S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS;
        // const FOUR_BYTE_LITERALS: u32 = 0x3; // todo S_4BYTE_LITERALS
        section.extend_from_slice(&INSTRUCTIONS_FLAG.to_le_bytes()); // flags
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // reserved1
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // reserved2
        section.extend_from_slice(&(0 as u32).to_le_bytes()); // reserved3

        section
    }
}

impl Executable for MachO {
    fn create(
        &mut self,
        data_sections: Vec<DataSection>,
        mut file: fs::File,
    ) -> std::io::Result<()> {
        let mut commands: Vec<Vec<u8>> = vec![];
        let zeropage_segment_cmd = self.create_segment_command(0, "__PAGEZERO", 0, 0, 0);

        commands.push(zeropage_segment_cmd);

        // let mut padded_data_bytes = data_sections.iter().fold(vec![], |mut acc, sect| {
        //     acc.extend(&sect.bytes);
        //     acc
        // });
        // let data_section_size = padded_data_bytes.len();

        // pad to a multiple of 8
        // while padded_data_bytes.len() % 0x1000 != 0 {
        //     padded_data_bytes.push(0);
        // }

        let data_size: u32 = data_sections
            .iter()
            .fold(0, |acc, section| acc + section.bytes.len() as u32);
        let mut padded_data_size = data_size;
        if padded_data_size % 0x1000 != 0 {
            padded_data_size += 0x1000 - (padded_data_size % 0x1000)
        }

        const SEGMENT_NAME: &str = "__TEXT";
        let mut code_segment_cmd = self.create_segment_command(
            padded_data_size + 0x1000,
            SEGMENT_NAME,
            DATA_SECTION_VIRTUAL_START_64,
            0, // executable.len() as u64, todo
            data_sections.len() as u32,
        );

        const PHYSICAL_DATA_START: u32 = 0x1000;
        let mut entry_vmaddr = 0;
        let mut vmaddr_offset: u32 = PHYSICAL_DATA_START;
        for data_section in &data_sections {
            let vmaddr_code: u64 = DATA_SECTION_VIRTUAL_START_64 + 0x1000;

            let section_name = if data_section.name == CODE_SECTION_NAME {
                entry_vmaddr = vmaddr_code;
                "__text"
            } else {
                &data_section.name
            };
            
            dbg!(vmaddr_code);
            dbg!(vmaddr_offset);
            let code_section = self.create_section(
                section_name,
                SEGMENT_NAME,
                vmaddr_code,
                data_section.bytes.len() as u64,
                vmaddr_offset as u32, // todo this should be based on the padded vmsize
            );

            code_segment_cmd.extend_from_slice(&code_section);
            vmaddr_offset += data_section.bytes.len() as u32;
        }

        commands.push(code_segment_cmd);

        let thread_cmd = self.create_thread_command(entry_vmaddr);
        commands.push(thread_cmd);

        let mut executable: Vec<u8> = vec![];
        let header = self.create_header(
            commands.len() as u32,
            commands
                .iter()
                .fold(0, |acc, command| acc + command.len() as u32),
        );

        executable.extend_from_slice(&header);
        for command in commands {
            executable.extend_from_slice(&command);
        }

        while executable.len() < PHYSICAL_DATA_START as usize {
            executable.push(0x00);
        }

        // executable.extend_from_slice(&padded_data_bytes);
        for data_section in &data_sections {
            executable.extend_from_slice(&data_section.bytes);
        }

        while executable.len() % 0x1000 != 0 {
            executable.push(0x00);
        }

        file.write_all(&executable)?;
        Ok(())
    }
}

#[cfg(test)]
mod test_mach_o {
    use super::*;

    #[test]
    fn test_mach_o_lengths() {
        let mut mach_o = MachO {};
        assert_eq!(mach_o.create_header(0, 0).len(), 32);
        assert_eq!(mach_o.create_segment_command(0, "test", 0, 0, 0).len(), 72);
        assert_eq!(mach_o.create_thread_command(0).len(), 4 * 4 + 21 * 8);
        assert_eq!(mach_o.create_section("test", "test", 0, 0, 0).len(), 80);
    }
}

pub struct ELF {}

impl ELF {
    fn create_string_table(&mut self, strings: &Vec<&String>) -> Vec<u8> {
        let mut table: Vec<u8> = vec![0x00]; // first byte is defined to be null
        for s in strings {
            table.extend(s.bytes());
            table.push(0x00);
        }

        table
    }

    fn create_section_header_entry(
        &mut self,
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
        &mut self,
        program_size: u32,
        data_section_sizes: &[u32],
        data_section_names: &[&String],
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
        section_header.append(&mut self.create_section_header_entry(
            0x00, SHT_NULL, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ));

        let mut next_section_virtual_start = DATA_SECTION_VIRTUAL_START_32;
        let mut next_section_physical_start = DATA_SECTION_PHYSICAL_START;
        for (index, size) in data_section_sizes.iter().enumerate() {
            section_header.append(&mut self.create_section_header_entry(
                strtab_index,
                SHT_PROGBITS,
                SHF_WRITE | SHF_ALLOC,
                next_section_virtual_start,
                next_section_physical_start,
                *size,
                0x00,
                0x00,
                0x01, // (no alignment constraint)
                0x00,
            ));

            // TODO program sizes are assumed to be 4KB
            next_section_physical_start += PAGE_SIZE;
            next_section_virtual_start += PAGE_SIZE;
            strtab_index += data_section_names[index].len() as u32 + 1;
        }

        // executable code
        section_header.append(&mut self.create_section_header_entry(
            strtab_index,
            SHT_PROGBITS,
            SHF_ALLOC | SHF_EXECINSTR,
            next_section_virtual_start,
            next_section_physical_start,
            program_size,
            0x00,
            0x00,
            0x01, // (no alignment constraint)
            0x00,
        ));

        // string table
        section_header.append(&mut self.create_section_header_entry(
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

    fn create_program_header_entry(
        &mut self,
        size: u32,
        offset: u32,
        virtual_address: u32,
        flags: u32,
    ) -> Vec<u8> {
        let mut entry: Vec<u8> = vec![];
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
        entry.extend_from_slice(&PT_LOAD.to_le_bytes());

        // p_offset
        entry.extend_from_slice(&offset.to_le_bytes());

        // p_vaddr
        entry.extend_from_slice(&virtual_address.to_le_bytes());

        // p_paddr (unspecified on System V, but seems to usually be virtual entry point)
        entry.extend_from_slice(&virtual_address.to_le_bytes());

        // p_filesz
        entry.extend_from_slice(&size.to_le_bytes());

        // p_memsz
        entry.extend_from_slice(&size.to_le_bytes());

        // p_flags
        entry.extend_from_slice(&flags.to_le_bytes());

        // p_align
        // align on 4KB
        entry.extend_from_slice(&(PAGE_SIZE as u32).to_le_bytes());

        entry
    }

    fn create_program_header(
        &mut self,
        program_size: u32,
        data_section_sizes: &Vec<u32>,
    ) -> Vec<u8> {
        const PF_X_R: u32 = 1 | (1 << 2);
        let mut program_header = self.create_program_header_entry(
            program_size,
            DATA_SECTION_PHYSICAL_START + PAGE_SIZE * data_section_sizes.len() as u32, // TODO this assumes data sections are 4KB
            DATA_SECTION_VIRTUAL_START_32 + PAGE_SIZE * data_section_sizes.len() as u32, // TODO this assumes data sections are 4KB
            PF_X_R,
        );

        let mut physical_address = DATA_SECTION_PHYSICAL_START;
        let mut virtual_address = DATA_SECTION_VIRTUAL_START_32;
        const PF_R_W: u32 = (1 << 2) | (1 << 1);
        for size in data_section_sizes.iter() {
            program_header.append(&mut self.create_program_header_entry(
                *size,
                physical_address,
                virtual_address,
                PF_R_W,
            ));

            // TODO program sizes are assumed to be 4KB
            physical_address += PAGE_SIZE;
            virtual_address += PAGE_SIZE;
        }

        program_header
    }

    fn create_elf_header(
        &mut self,
        number_of_program_headers: u32,
        number_of_sections: u32,
    ) -> Vec<u8> {
        const END_ELF_HEADER: u32 = 0x34;
        const PROGRAM_HEADER_SIZE: u32 = 32;
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
        // TODO this assumes 4 KB data sections
        // -3 because string table appears in the first page and null delimiter
        // and code don't offset the virtual entry point
        header.extend_from_slice(
            &(DATA_SECTION_VIRTUAL_START_32 + (number_of_sections - 3) * PAGE_SIZE).to_le_bytes(),
        );

        // Start of program header table (immediately after this header)
        header.extend_from_slice(&END_ELF_HEADER.to_le_bytes());

        // e_shoff: Start of section header table
        let program_header_table_size: u32 = number_of_program_headers * PROGRAM_HEADER_SIZE;
        header.extend_from_slice(&(END_ELF_HEADER + program_header_table_size).to_le_bytes());

        // eflags
        header.append(&mut vec![0x00; 4]);

        // Size of this header
        header.append(&mut vec![52, 0x00]);

        // e_phentsize: size of a program header table entry
        header.append(&mut vec![PROGRAM_HEADER_SIZE as u8, 0x00]);

        // e_phnum: number of entries in program header table
        header.append(&mut vec![number_of_program_headers as u8, 0x00]);

        // e_shentsize: size of a section header table entry
        header.append(&mut vec![40, 0x00]);

        // e_shnum: number of entries in section header table
        header.append(&mut vec![number_of_sections as u8, 0x00]);

        // e_shstrndx: index of section header table entry that contains section names
        header.append(&mut vec![(number_of_sections - 1) as u8, 0x00]);

        header
    }
}

#[cfg(test)]
mod test_elf {
    use super::*;

    #[test]
    fn test_elf_header_length() {
        let mut elf = ELF {};
        assert_eq!(elf.create_elf_header(1, 3).len(), 52);
    }

    #[test]
    fn test_section_header_length() {
        const BYTES_PER_FIELD: usize = 4;
        const FIELDS_PER_ENTRY: usize = 10;
        const ENTRIES: usize = 3;
        let mut elf = ELF {};

        assert_eq!(
            elf.create_section_header(0, &vec![], &vec![], 0).len(),
            BYTES_PER_FIELD * FIELDS_PER_ENTRY * ENTRIES
        );
    }

    #[test]
    fn test_program_header_length() {
        let mut elf = ELF {};
        assert_eq!(elf.create_program_header(0, &vec![]).len(), 8 * 4);
    }
}

impl Executable for ELF {
    fn create(
        &mut self,
        mut data_sections: Vec<DataSection>,
        mut file: fs::File,
    ) -> std::io::Result<()> {
        // + 2 for string table and null sentinel
        let elf_header =
            self.create_elf_header(data_sections.len() as u32, data_sections.len() as u32 + 2);
        let total_sections = data_sections.len();
        let program = data_sections.remove(total_sections - 1).bytes;

        let data_section_sizes = data_sections
            .iter()
            .map(|section| section.bytes.len() as u32)
            .collect();
        let program_header = self.create_program_header(program.len() as u32, &data_section_sizes);

        let data_section_names = data_sections.iter().map(|section| &section.name).collect();
        let mut string_table = self.create_string_table(&data_section_names);

        // add str name for code and strtab at end of table
        string_table.extend(CODE_SECTION_NAME.bytes());
        string_table.push(0x00);
        string_table.extend(STRTAB_SECTION_NAME.bytes());
        string_table.push(0x00);

        let section_header = self.create_section_header(
            program.len() as u32,
            &data_section_sizes,
            &data_section_names,
            string_table.len() as u32,
        );

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

        let padding = vec![
            0;
            DATA_SECTION_PHYSICAL_START as usize
                - STRTABLE_PHYSICAL_ENTRY_POINT as usize
                - string_table.len()
        ];
        file.write_all(&padding)?;

        // insert data sections
        // DATA_SECTION_PHYSICAL_START
        for section in data_sections.iter() {
            let data = &section.bytes;
            file.write_all(&data)?;

            // pad current data section
            let padding = vec![0; PAGE_SIZE as usize - (data.len() % PAGE_SIZE as usize)];
            file.write_all(&padding)?;
        }

        file.write_all(&program)?;

        Ok(())
    }
}
