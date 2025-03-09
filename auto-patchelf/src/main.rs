// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

mod cache;
mod cli;
mod concurrency;
mod elf;
mod misc;
mod state;

use eyre::{eyre, Context, Result};
use glob::Pattern;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
};

use crate::{
    cache::LibraryCache,
    cli::{Cli, PatchConfig},
    concurrency::SharedHandle,
    elf::{machine_to_str, osabi_are_compatible, osabi_to_string, ElfFile},
    misc::{glob, read_file},
    state::DirState,
};

const DEFAULT_BINTOOLS: &str = "@defaultBintools@";

#[derive(Debug, Clone)]
struct Dependency {
    file: PathBuf,
    name: PathBuf,
    found: bool,
}

/// Patches a single ELF file
fn auto_patchelf_file(
    args: &PatchConfig,
    path: &Path,
    library_computation: &SharedHandle<LibraryCache>,
    interpreter_path: &Path,
    interpreter: &ElfFile,
    libc_lib: &Path,
) -> Result<Vec<Dependency>> {
    let mut dependencies = Vec::new();

    let content = read_file(path).unwrap();
    let elf_file: ElfFile = match ElfFile::new(&content) {
        Ok(elf) => elf,
        Err(_) => return Ok(dependencies),
    };

    // Skip files that don't need patching
    if elf_file.is_static_executable() {
        println!(
            "skipping {} because it is statically linked",
            path.display()
        );
        return Ok(dependencies);
    }

    if elf_file.has_program_headers() {
        println!("skipping {} because it contains no segment", path.display());
        return Ok(dependencies);
    }

    if interpreter.get_arch() != elf_file.get_arch() {
        println!(
            "skipping {} because its architecture ({}) differs from target ({})",
            path.display(),
            machine_to_str(elf_file.get_arch()),
            machine_to_str(interpreter.get_arch())
        );
        return Ok(dependencies);
    }

    if !osabi_are_compatible(interpreter.get_osabi(), elf_file.get_osabi()) {
        println!(
            "skipping {} because its OS ABI ({}) is not compatible with target ({})",
            path.display(),
            osabi_to_string(elf_file.get_osabi()),
            osabi_to_string(interpreter.get_osabi())
        );
        return Ok(dependencies);
    }

    let file_is_dynamic_executable = elf_file.is_dynamic_executable();
    let file_dependencies = elf_file.get_dependencies();

    let mut rpath = Vec::new();

    // Set interpreter for executables
    if file_is_dynamic_executable {
        println!("setting interpreter of {}", path.display());

        let output = Command::new("patchelf")
            .arg("--set-interpreter")
            .arg(interpreter_path)
            .arg(path)
            .args(&args.extra_args)
            .output()?;

        if !output.status.success() {
            let err =
                core::str::from_utf8(&output.stderr).unwrap_or("Could not format command output");
            return Err(eyre!(
                "Failed to set interpreter for {}, output: {}",
                path.display(),
                err
            ));
        }

        rpath.extend(args.runtime_dependencies.iter().cloned());
    }

    println!("searching for dependencies of {}", path.display());

    let library_cache = library_computation.get_result()?;

    // Process dependencies
    for dep in file_dependencies {
        let mut was_found = false;

        for candidate in &dep {
            // This loop determines which candidate for a given
            // dependency can be found, and how. There may be multiple
            // candidates for a dep because of '.note.dlopen'
            // dependencies.
            //
            // 1. If a candidate is an absolute path, it is already a
            //    valid dependency if that path exists, and nothing needs
            //    to be done. It should be an error if that path does not exist.
            // 2. If a candidate is found within libc, it should be dropped
            //    and resolved automatically by the dynamic linker, unless
            //    keep_libc is enabled.
            // 3. If a candidate is found in our library dependencies, that
            //    dependency should be added to rpath.
            // 4. If all of the above fail, libc dependencies should still be
            //    considered found. This is in contrast to step 2, because
            //    enabling keep_libc should allow libc to be found in step 3
            //    if possible to preserve its presence in rpath.
            //
            // These conditions are checked in this order, because #2
            // and #3 may both be true. In that case, we still want to
            // add the dependency to rpath, as the original binary
            // presumably had it and this should be preserved.

            let is_libc = libc_lib.join(candidate).is_file();

            #[allow(clippy::if_same_then_else)]
            if candidate.is_absolute() && candidate.is_file() {
                was_found = true;
                break;
            } else if is_libc && !args.keep_libc {
                was_found = true;
                break;
            } else if let Some(candidate_name) = candidate.file_name().and_then(|n| n.to_str()) {
                if let Some(found_dependency) = library_cache.find_dependency(
                    candidate_name,
                    elf_file.get_arch(),
                    elf_file.get_osabi(),
                ) {
                    rpath.push(found_dependency.clone());
                    dependencies.push(Dependency {
                        file: path.to_path_buf(),
                        name: candidate.clone(),
                        found: true,
                    });
                    println!(
                        " {} -> found: {}",
                        candidate.display(),
                        found_dependency.display()
                    );
                    was_found = true;
                    break;
                }
            } else if is_libc && args.keep_libc {
                was_found = true;
                break;
            }
        }

        if !was_found {
            let dep_name = if dep.len() == 1 {
                dep[0].clone()
            } else {
                let names: Vec<String> = dep.iter().map(|p| p.display().to_string()).collect();
                PathBuf::from(format!("any({})", names.join(", ")))
            };

            dependencies.push(Dependency {
                file: path.to_path_buf(),
                name: dep_name.clone(),
                found: false,
            });

            println!(" {} -> not found!", dep_name.display());
        }
    }

    rpath.extend(args.append_rpaths.iter().cloned());

    // Deduplicate rpath entries
    let mut unique_paths = HashMap::new();
    for path in rpath {
        let path_str = path.to_string_lossy().to_string();
        unique_paths.entry(path_str).or_insert(path);
    }

    let deduped_rpath: Vec<_> = unique_paths.keys().cloned().collect();

    if !deduped_rpath.is_empty() {
        let rpath_str = deduped_rpath.join(":");
        println!("setting RPATH to: {rpath_str}");

        Command::new("patchelf")
            .arg("--set-rpath")
            .arg(&rpath_str)
            .arg(path)
            .args(&args.extra_args)
            .status()
            .ok();
    }

    Ok(dependencies)
}

/// Main auto-patchelf function
fn auto_patchelf(
    cli: &Cli,
    interpreter: &ElfFile,
    interpreter_path: &Path,
    libc_lib: &Path,
) -> Result<()> {
    if cli.patch.paths.is_empty() {
        return Err(eyre!("No paths to patch, stopping."));
    }

    let add_existing = cli.libraries.add_existing;
    let recurse = cli.patch.recurse;
    let paths = cli.patch.paths.clone();
    let libraries = cli.libraries.libraries.clone();
    let cache_computation = SharedHandle::new(thread::spawn(move || {
        let mut library_cache = LibraryCache::new();

        // Add all shared objects of the current output path to the cache
        if add_existing {
            library_cache.populate_cache(&paths, recurse)?;
        }

        library_cache.populate_cache(&libraries, false)?;
        Ok(library_cache)
    }));

    let mut all_dependencies = Vec::new();

    // Process all files
    for path in &cli.patch.paths {
        let mut state = DirState::deserialize(path)?;

        for file_path in glob(path, "*", cli.patch.recurse)? {
            let file_path = file_path?;
            let cache_path = file_path.strip_prefix(path)?;

            if file_path.is_symlink() || !file_path.is_file() {
                continue; // We care about regular files only, and we don't want to traverse symlinks
            }

            let mut buf = [0u8; 4];
            let read = File::open(&file_path)?.read_exact(&mut buf);
            if read.is_err() || buf != [0x7f, 0x45, 0x4c, 0x46] {
                continue; // We care about elf files only
            }

            let mtime = file_path.metadata()?.mtime();

            if state.up_to_date(cache_path, mtime) {
                continue;
            }

            auto_patchelf_file(
                &cli.patch,
                &file_path,
                &cache_computation,
                interpreter_path,
                interpreter,
                libc_lib,
            )
            .inspect_err(|e| {
                println!("Coulld not patch file: {e}");
            })
            .and_then(|deps| {
                let mtime = file_path.metadata()?.mtime();
                state.update(cache_path.to_owned(), mtime);
                all_dependencies.extend(deps);
                Ok(())
            })
            .unwrap_or_default();
        }

        state.serialize()?;
    }

    // Check for missing dependencies
    let missing: Vec<&Dependency> = all_dependencies.iter().filter(|dep| !dep.found).collect();

    println!(
        "auto-patchelf: {} dependencies could not be satisfied",
        missing.len()
    );

    let mut failure = false;

    for dep in missing {
        let mut ignored = false;

        if let Some(name) = dep.name.file_name().and_then(|n| n.to_str()) {
            for pattern in &cli.patch.ignore_missing {
                if Pattern::new(pattern)
                    .map(|p| p.matches(name))
                    .unwrap_or(false)
                {
                    println!(
                        "warn: auto-patchelf ignoring missing {} wanted by {}",
                        dep.name.display(),
                        dep.file.display()
                    );
                    ignored = true;
                    break;
                }
            }
        }

        if !ignored {
            println!(
                "error: auto-patchelf could not satisfy dependency {} wanted by {}",
                dep.name.display(),
                dep.file.display()
            );
            failure = true;
        }
    }

    if failure {
        return Err(eyre!(
            "auto-patchelf failed to find all the required dependencies.\n\
            Add the missing dependencies to --libs or use \
            `--ignore-missing=\"foo.so.1 bar.so etc.so\"`."
        ));
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse()?;
    println!("automatically fixing dependencies for ELF files");

    // Get interpreter information
    let nix_bintools = env::var("NIX_BINTOOLS").unwrap_or_else(|_| DEFAULT_BINTOOLS.to_string());
    let nix_support = PathBuf::from(nix_bintools).join("nix-support");

    let interpreter_path =
        PathBuf::from(fs::read_to_string(nix_support.join("dynamic-linker"))?.trim());
    let libc_lib =
        PathBuf::from(fs::read_to_string(nix_support.join("orig-libc"))?.trim()).join("lib");

    let content = read_file(&interpreter_path)
        .wrap_err_with(|| format!("Failed to read file {}", interpreter_path.display(),))?;
    let interpreter = ElfFile::new(&content).wrap_err_with(|| {
        format!(
            "Failed to parse dynamic linker properties from {}",
            interpreter_path.display(),
        )
    })?;

    if !interpreter_path.exists() || !libc_lib.exists() {
        return Err(eyre!("Failed to parse dynamic linker properties."));
    }

    // Run the patching process
    auto_patchelf(&args, &interpreter, &interpreter_path, &libc_lib)
}
