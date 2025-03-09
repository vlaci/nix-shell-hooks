// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::path::PathBuf;

use eyre::Result;
use goblin::elf::{dynamic, header, program_header, Elf};
use miniserde::{json, Deserialize};

pub(crate) use goblin::elf::header::machine_to_str;

pub(crate) struct ElfFile<'a> {
    content: &'a [u8],
    elf: Elf<'a>,
}

pub(crate) type Arch = u16;
pub(crate) type OsAbi = u8;

impl<'a> ElfFile<'a> {
    pub(crate) fn new(content: &'a Vec<u8>) -> Result<Self> {
        let elf = Elf::parse(content)?;
        Ok(Self { content, elf })
    }
    pub(crate) fn get_arch(&self) -> Arch {
        self.elf.header.e_machine
    }

    pub(crate) fn get_osabi(&self) -> OsAbi {
        self.elf.header.e_ident[header::EI_OSABI]
    }

    pub(crate) fn has_program_headers(&self) -> bool {
        self.elf.program_headers.is_empty()
    }

    /// Checks if an ELF file is a statically linked executable
    pub(crate) fn is_static_executable(&self) -> bool {
        self.elf.header.e_type == header::ET_EXEC
            && !self
                .elf
                .program_headers
                .iter()
                .any(|ph| ph.p_type == program_header::PT_INTERP)
    }

    /// Checks if an ELF file is a dynamically linked executable
    pub(crate) fn is_dynamic_executable(&self) -> bool {
        self.elf
            .program_headers
            .iter()
            .any(|ph| ph.p_type == program_header::PT_INTERP)
    }

    /// Gets the RPATH from the dynamic section
    pub(crate) fn get_rpath(&self) -> Vec<String> {
        if let Some(dynamics) = &self.elf.dynamic {
            // First try RUNPATH
            for dynamic in &dynamics.dyns {
                if dynamic.d_tag == dynamic::DT_RUNPATH {
                    if let Some(runpath) = self.elf.dynstrtab.get_at(dynamic.d_val as usize) {
                        return runpath.split(':').map(String::from).collect();
                    }
                }
            }

            // Fall back to RPATH
            for dynamic in &dynamics.dyns {
                if dynamic.d_tag == dynamic::DT_RPATH {
                    if let Some(rpath) = self.elf.dynstrtab.get_at(dynamic.d_val as usize) {
                        return rpath.split(':').map(String::from).collect();
                    }
                }
            }
        }

        Vec::with_capacity(0)
    }

    /// Gets the dynamic dependencies of an ELF file
    pub(crate) fn get_dependencies(&self) -> Vec<Vec<PathBuf>> {
        let mut dependencies = Vec::new();

        if let Some(dynamics) = &self.elf.dynamic {
            for dynamic in &dynamics.dyns {
                if dynamic.d_tag == dynamic::DT_NEEDED {
                    if let Some(name) = self.elf.dynstrtab.get_at(dynamic.d_val as usize) {
                        dependencies.push(vec![PathBuf::from(name)]);
                    }
                }
            }
        }

        // Find .note.dlopen section
        // See https://systemd.io/ELF_DLOPEN_METADATA/
        if let Some(notes) = &self
            .elf
            .iter_note_sections(self.content, Some(".note.dlopen"))
        {
            for note in &notes.iters {
                let Ok(text) = String::from_utf8(note.data.into()) else {
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

        dependencies
    }
}

#[derive(Deserialize)]
struct DlOpen {
    soname: Vec<String>,
}

/// Gets OS ABI information from the ELF header
pub(crate) fn osabi_to_string(abi: OsAbi) -> String {
    match abi {
        header::ELFOSABI_SYSV => "ELFOSABI_SYSV".to_string(),
        header::ELFOSABI_HPUX => "ELFOSABI_HPUX".to_string(),
        header::ELFOSABI_NETBSD => "ELFOSABI_NETBSD".to_string(),
        header::ELFOSABI_LINUX => "ELFOSABI_LINUX".to_string(),
        header::ELFOSABI_SOLARIS => "ELFOSABI_SOLARIS".to_string(),
        header::ELFOSABI_AIX => "ELFOSABI_AIX".to_string(),
        header::ELFOSABI_IRIX => "ELFOSABI_IRIX".to_string(),
        header::ELFOSABI_FREEBSD => "ELFOSABI_FREEBSD".to_string(),
        header::ELFOSABI_TRU64 => "ELFOSABI_TRU64".to_string(),
        header::ELFOSABI_MODESTO => "ELFOSABI_MODESTO".to_string(),
        header::ELFOSABI_OPENBSD => "ELFOSABI_OPENBSD".to_string(),
        abi => format!("unknown_{abi}"),
    }
}

/// Checks if two OS ABIs are compatible
pub(crate) fn osabi_are_compatible(wanted: OsAbi, got: OsAbi) -> bool {
    if wanted == header::ELFOSABI_SYSV || got == header::ELFOSABI_SYSV {
        return true; // System V ABI is broadly compatible
    }

    wanted == got // Otherwise require exact match
}
