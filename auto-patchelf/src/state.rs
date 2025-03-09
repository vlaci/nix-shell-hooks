// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use bincode::Options;
use eyre::{bail, Result};

use crate::misc::path_string;

type MTime = i64;
type Cache = HashMap<PathBuf, MTime>;

pub(crate) struct DirState {
    file: File,
    cache: Cache,
}

impl DirState {
    const VERSION: u32 = 1;
    pub(crate) fn deserialize(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .read(true)
            .open(path.as_ref().join(".auto-patchelf.state"))?;

        let cache = Self::deserialize_cache(&mut file)
            .inspect_err(|err| {
                println!(
                    "Unable to load cache file from {} {}",
                    path_string(&path),
                    err
                );
            })
            .unwrap_or_default();

        Ok(Self { file, cache })
    }

    fn deserialize_cache(file: &mut File) -> Result<Cache> {
        let deserializer = bincode::options()
            .with_fixint_encoding()
            .with_limit(32 << 20);
        let version_size = deserializer.serialized_size(&Self::VERSION).unwrap() as _;
        let mut version_buf = vec![0; version_size];
        file.read_exact(&mut version_buf)?;
        let version: u32 = deserializer.deserialize(&version_buf)?;
        if version != Self::VERSION {
            bail!("Invalid version {}", version)
        }

        Ok(deserializer.deserialize_from(file)?)
    }

    pub(crate) fn serialize(mut self) -> Result<()> {
        self.file.rewind()?;
        self.file.set_len(0)?;
        bincode::serialize_into(&mut self.file, &Self::VERSION)?;
        bincode::serialize_into(&mut self.file, &self.cache)?;
        Ok(())
    }

    pub(crate) fn up_to_date(&self, path: impl AsRef<Path>, mtime: MTime) -> bool {
        self.cache
            .get(path.as_ref())
            .is_some_and(|&entry| mtime == entry)
    }

    pub(crate) fn update(&mut self, path: PathBuf, mtime: MTime) {
        self.cache
            .entry(path)
            .and_modify(|entry| *entry = mtime)
            .or_insert(mtime);
    }
}
