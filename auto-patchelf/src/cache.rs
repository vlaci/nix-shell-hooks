// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use eyre::Result;

use crate::{
    elf::{osabi_are_compatible, Arch, ElfFile, OsAbi},
    misc::{glob, read_file},
};

/// Library cache to avoid duplicate scanning
pub(crate) struct LibraryCache {
    cached_paths: HashSet<PathBuf>,
    soname_cache: HashMap<(String, Arch), Vec<(PathBuf, OsAbi)>>,
}

impl LibraryCache {
    pub(crate) fn new() -> Self {
        Self {
            cached_paths: HashSet::new(),
            soname_cache: HashMap::new(),
        }
    }

    /// Populates the cache with libraries from specified paths
    pub(crate) fn populate_cache(&mut self, initial: &[PathBuf], recursive: bool) -> Result<()> {
        let mut lib_dirs = initial.to_vec();

        while !lib_dirs.is_empty() {
            let lib_dir = lib_dirs.remove(0);

            if self.cached_paths.contains(&lib_dir) {
                continue;
            }

            self.cached_paths.insert(lib_dir.clone());

            for path in glob(&lib_dir, "*.so*", recursive)?.flatten() {
                if !path.is_file() {
                    continue;
                }

                // Resolve symlinks optimally
                let resolved = match path.canonicalize() {
                    Ok(p) if p.file_name() == path.file_name() => p,
                    _ => path.clone(),
                };
                let content = read_file(&path)?;
                if let Ok(elf) = ElfFile::new(&content) {
                    // Add RPATH directories to search list
                    let rpath: Vec<PathBuf> = elf
                        .get_rpath()
                        .iter()
                        .filter(|p| !p.is_empty() && !p.contains("$ORIGIN"))
                        .map(PathBuf::from)
                        .collect();

                    lib_dirs.extend(rpath);

                    // Cache this library
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let key = (name.to_string(), elf.get_arch());
                        self.soname_cache.entry(key).or_default().push((
                            resolved.parent().unwrap_or(Path::new("")).to_path_buf(),
                            elf.get_osabi(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Finds a dependency in the cache
    pub(crate) fn find_dependency(
        &self,
        soname: &str,
        soarch: Arch,
        soabi: OsAbi,
    ) -> Option<PathBuf> {
        self.soname_cache
            .get(&(soname.to_string(), soarch))
            .and_then(|libs| {
                libs.iter()
                    .find(|(_, libabi)| osabi_are_compatible(soabi, *libabi))
                    .map(|(lib, _)| lib.clone())
            })
    }
}
