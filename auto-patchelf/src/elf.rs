// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    io::{Read, Seek},
    path::PathBuf,
};

use elf::{abi, endian::AnyEndian, note::Note, ElfStream};
use eyre::Result;
use miniserde::{json, Deserialize};

pub(crate) use elf::to_str::e_machine_to_str;
pub(crate) use elf::to_str::e_osabi_to_str;

pub(crate) struct ElfFile<S: Read + Seek> {
    elf: ElfStream<AnyEndian, S>,
}

pub(crate) type Arch = u16;
pub(crate) type OsAbi = u8;

impl<S: Read + Seek> ElfFile<S> {
    pub(crate) fn new(reader: S) -> Result<Self> {
        let stream = ElfStream::open_stream(reader)?;
        Ok(Self { elf: stream })
    }
    pub(crate) fn get_arch(&self) -> Arch {
        self.elf.ehdr.e_machine
    }

    pub(crate) fn get_osabi(&self) -> OsAbi {
        self.elf.ehdr.osabi
    }

    pub(crate) fn has_program_headers(&self) -> bool {
        !self.elf.segments().is_empty()
    }

    /// Checks if an ELF file is a statically linked executable
    pub(crate) fn is_static_executable(&self) -> bool {
        self.elf.ehdr.e_type == abi::ET_EXEC
            && !self
                .elf
                .segments()
                .iter()
                .any(|ph| ph.p_type == abi::PT_INTERP)
    }

    /// Checks if an ELF file is a dynamically linked executable
    pub(crate) fn is_dynamic_executable(&self) -> bool {
        self.elf
            .segments()
            .iter()
            .any(|ph| ph.p_type == abi::PT_INTERP)
    }

    pub(crate) fn parse(&mut self) -> Result<Option<ParsedElf>> {
        let dynamics = self.elf.dynamic()?;

        let Some(dynamics) = dynamics else {
            return Ok(None);
        };

        let mut dt_runpath = None;
        let mut dt_rpath = None;
        let mut dt_needed = None;

        for d in dynamics.iter() {
            match d.d_tag {
                abi::DT_RUNPATH => dt_runpath = Some(d.d_val()),
                abi::DT_RPATH => dt_rpath = Some(d.d_val()),
                abi::DT_NEEDED => dt_needed = Some(d.d_val()),
                _ => (),
            }
        }

        let (symtab, strings) = self.elf.dynamic_symbol_table()?.unwrap();

        // First try RUNPATH
        let rpath = if let Some(path) = dt_runpath.or(dt_rpath) {
            strings
                .get(path as _)
                .map_or_else(|e| Vec::new(), |p| p.split(':').map(String::from).collect())
        } else {
            Vec::new()
        };

        let mut dependencies = Vec::new();

        if let Some(needed) = dt_needed {
            if let Ok(name) = strings.get(needed as usize) {
                dependencies.push(vec![PathBuf::from(name)]);
            }
        }

        // Find .note.dlopen section
        // See https://systemd.io/ELF_DLOPEN_METADATA/
        if let Some(shdr) = self.elf.section_header_by_name(".note.dlopen")? {
            let shdr = *shdr;
            let notes = self.elf.section_data_as_notes(&shdr)?;
            for note in notes {
                let Note::Unknown(data) = note else {
                    continue;
                };
                let Ok(text) = String::from_utf8(data.desc.into()) else {
                    continue;
                };
                let Ok(dlopen) = json::from_str::<DlOpen>(&text) else {
                    continue;
                };
                if !dlopen.soname.is_empty() {
                    dependencies.push(dlopen.soname.into_iter().map(PathBuf::from).collect());
                }
            }
        }

        Ok(Some(ParsedElf {
            rpath,
            dependencies,
        }))
    }
}

pub(crate) struct ParsedElf {
    pub(crate) rpath: Vec<String>,
    pub(crate) dependencies: Vec<Vec<PathBuf>>,
}

#[derive(Deserialize)]
struct DlOpen {
    soname: Vec<String>,
}

/// Checks if two OS ABIs are compatible
pub(crate) fn osabi_are_compatible(wanted: OsAbi, got: OsAbi) -> bool {
    if wanted == abi::ELFOSABI_SYSV || got == abi::ELFOSABI_SYSV {
        return true; // System V ABI is broadly compatible
    }

    wanted == got // Otherwise require exact match
}
