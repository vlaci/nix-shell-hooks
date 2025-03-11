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
    pub(crate) fn new(content: &'a [u8]) -> Result<Self> {
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
        !self.elf.program_headers.is_empty()
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
        for note in self
            .elf
            .iter_note_sections(&self.content, Some(".note.dlopen"))
            .into_iter()
            .flatten()
        {
            let note = note.unwrap();

            let Ok(text) = std::str::from_utf8(note.desc) else {
                continue;
            };
            let text = text.trim_end_matches('\0');
            let Ok(dlopens) = json::from_str::<Vec<DlOpen>>(&text) else {
                continue;
            };
            for dlopen in dlopens {
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! asset {
        ($fname:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets/", $fname)
        };
    }
    #[test]
    fn test_parsing() {
        let content = include_bytes!(asset!("pam_systemd_home.so"));

        let elf = ElfFile::new(content).unwrap();

        assert_eq!(elf.get_arch(), header::EM_X86_64);
        assert_eq!(elf.get_osabi(), header::ELFOSABI_NONE);
        assert!(elf.has_program_headers());
        assert!(!elf.is_static_executable());
        assert!(!elf.is_dynamic_executable());

        assert_eq!(
            elf.get_rpath(),
            vec![
                "/nix/store/0szrc79hm06rprwd4v5lg80fwg4sn2wj-libxcrypt-4.4.36/lib",
                "/nix/store/mhrs2z02f605vm22xkwkqci14myz5ahc-linux-pam-1.6.1/lib",
                "/nix/store/h7zcxabfxa7v5xdna45y2hplj31ncf8a-glibc-2.40-36/lib",
                "/nix/store/7si4bninwpbdhz387ik0ajwr7yi5pf1s-libcap-2.73-lib/lib",
                "/nix/store/b96snfpcdisxf4s054wmq48l0jrjl562-util-linux-minimal-2.39.4-lib/lib",
                "/nix/store/5926bc2bqkr0882p4m6yq3wy5brnbwlv-openssl-3.3.2/lib",
                "/nix/store/ii1fdbyzymw9vj2ywxrmz34wbnlg4fb6-libidn2-2.3.7/lib",
                "/nix/store/4ab37gxv8n45c4hx1g5939w88y92chvx-p11-kit-0.25.5/lib",
                "/nix/store/60z71xaimq5giq0844pkqcfln6xzcgi0-tpm2-tss-4.1.3/lib",
                "/nix/store/8v985qha35alxwgn5s7m6632fk6pcjp2-cryptsetup-2.7.5/lib"
            ]
        );

        // dynamic dependencies
        // [
        //   {
        //     "feature": "idn",
        //     "description": "Support for internationalized domain names",
        //     "priority": "suggested",
        //     "soname": [
        //       "libidn2.so.0"
        //     ]
        //   }
        // ]
        // [
        //   {
        //     "feature": "p11 -kit",
        //     "description": "Support for PKCS11 hardware tokens",
        //     "priority": "suggested",
        //     "soname": [
        //       "libp11-kit.so.0"
        //     ]
        //   }
        // ]
        // [
        //   {
        //     "feature": "tpm",
        //     "description": "Support for TPM",
        //     "priority": "suggested",
        //     "soname ": [
        //       "libtss2-mu.so.0"
        //     ]
        //   }
        // ]
        // [
        //   {
        //     "feature": "tpm{",
        //     "description": "Support for TPM",
        //     "priority": "suggested",
        //     "soname": [
        //       "libtss2-rc.so.0"
        //     ]
        //   }
        // ]
        // [
        //   {
        //     "feature": "tpm",
        //     "description": "Support for TPM",
        //     "priority": "suggested",
        //     "soname ": [
        //       "libtss2-esys.so.0"
        //     ]
        //   }
        // ]
        // [
        //   {
        //     "feature": "cryptsetup",
        //     "descriptin": "Support for disk encryption,integrity, and authentication",
        //     "priority": "suggested",
        //     "soname": [
        //       "libcryptsetup.so.12"
        //     ]
        //   }
        // ]

        assert_eq!(
            elf.get_dependencies(),
            vec![
                vec![PathBuf::from("libcrypt.so.2")],
                vec![PathBuf::from("libpam.so.0")],
                vec![PathBuf::from("libm.so.6")],
                vec![PathBuf::from("libcap.so.2")],
                vec![PathBuf::from("libblkid.so.1")],
                vec![PathBuf::from("libmount.so.1")],
                vec![PathBuf::from("libcrypto.so.3")],
                vec![PathBuf::from("libc.so.6")],
                vec![PathBuf::from("ld-linux-x86-64.so.2")],
                vec![PathBuf::from("libidn2.so.0")],
                vec![PathBuf::from("libp11-kit.so.0")],
                vec![PathBuf::from("libtss2-mu.so.0")],
                vec![PathBuf::from("libtss2-rc.so.0")],
                vec![PathBuf::from("libtss2-esys.so.0")],
                vec![PathBuf::from("libcryptsetup.so.12")]
            ]
        );
    }
}
