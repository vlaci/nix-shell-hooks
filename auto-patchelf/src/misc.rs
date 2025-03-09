// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use eyre::Result;
use glob::Paths;
use std::{fs::File, io::Read, path::Path};

pub(crate) fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

pub(crate) fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(path)?.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub(crate) fn glob(path: &Path, pattern: &str, recursive: bool) -> Result<Paths> {
    let pattern = if recursive {
        format!("{}/**/{}", path.display(), pattern)
    } else {
        format!("{}/{}", path.display(), pattern)
    };
    Ok(glob::glob(&pattern)?)
}
