// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::path::PathBuf;

pub(crate) struct Cli {
    pub(crate) patch: PatchConfig,

    pub(crate) libraries: LibrariesConfig,
}

pub(crate) struct PatchConfig {
    pub(crate) ignore_missing: Vec<String>,
    pub(crate) recurse: bool,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) runtime_dependencies: Vec<PathBuf>,
    pub(crate) append_rpaths: Vec<PathBuf>,
    pub(crate) keep_libc: bool,
    pub(crate) extra_args: Vec<String>,
}

pub(crate) struct LibrariesConfig {
    pub(crate) libraries: Vec<PathBuf>,
    pub(crate) add_existing: bool,
}

/// Parse zero or more arguments
macro_rules! many0 {
    ($parser:expr) => {{
        let Ok(values) = $parser.values() else {
            continue;
        };
        values.map(|v| v.parse()).collect::<Result<_, _>>()?
    }};
}

impl Cli {
    pub(crate) fn parse() -> Result<Self, lexopt::Error> {
        use lexopt::prelude::*;

        let mut ignore_missing = Vec::new();
        let mut recurse = true;
        let mut paths = Vec::new();
        let mut libraries = Vec::new();
        let mut runtime_dependencies = Vec::new();
        let mut append_rpaths = Vec::new();
        let mut keep_libc = false;
        let mut add_existing = true;
        let mut extra_args = Vec::new();

        let mut parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Long("ignore-missing") => {
                    ignore_missing = many0!(parser);
                }
                Long("no-recurse") => {
                    recurse = false;
                }
                Long("paths") => {
                    paths = many0!(parser);
                }
                Long("libs") => {
                    libraries = many0!(parser);
                }
                Long("runtime-dependencies") => {
                    runtime_dependencies = many0!(parser);
                }
                Long("append-rpaths") => {
                    append_rpaths = many0!(parser);
                }
                Long("keep-libc") => {
                    keep_libc = true;
                }
                Long("ignore-existing") => {
                    add_existing = false;
                }
                Long("extra-args") => {
                    extra_args = many0!(parser);
                }
                Short('h') | Long("help") => {
                    println!(
                        r#"automatically fixing dependencies for ELF files

auto-patchelf tries as hard as possible to patch the provided binary files by looking for compatible libraries in the provided paths.

Usage: auto-patchelf [OPTIONS] --paths [<PATHS>...]

Options:
      --ignore-missing [<IGNORE_MISSING>...]
          Do not fail when some dependencies are not found
      --no-recurse
          Disable the recursive traversal of paths to patch
      --paths [<PATHS>...]
          Paths whose content needs to be patched. Single files and directories are accepted. Directories are traversed recursively by default
      --runtime-dependencies [<RUNTIME_DEPENDENCIES>...]
          Paths to prepend to the runtime path of executable binaries. Subject to deduplication, which may imply some reordering
      --append-rpaths [<APPEND_RPATHS>...]
          Paths to append to all runtime paths unconditionally
      --keep-libc
          Attempt to search for and relink libc dependencies
      --extra-args [<EXTRA_ARGS>...]
          Extra arguments to pass to patchelf. This argument should always come last
      --libs [<LIBRARIES>...]
          Paths where libraries are searched for. Single files and directories are accepted. Directories are not searched recursively
      --ignore-existing
          Do not add the existing rpaths of the patched files to the list of directories to search for dependencies
  -h, --help
          Print help
"#
                    );
                    std::process::exit(0);
                }
                _ => return Err(arg.unexpected()),
            }
        }

        Ok(Self {
            patch: PatchConfig {
                ignore_missing,
                recurse,
                paths,
                runtime_dependencies,
                append_rpaths,
                keep_libc,
                extra_args,
            },
            libraries: LibrariesConfig {
                libraries,
                add_existing,
            },
        })
    }
}
