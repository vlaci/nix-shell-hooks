// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use eyre::Result;
use glob::Paths;
use std::path::Path;

pub(crate) fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

pub(crate) fn glob(path: &Path, pattern: &str, recursive: bool) -> Result<Paths> {
    let pattern = if recursive {
        format!("{}/**/{}", path.display(), pattern)
    } else {
        format!("{}/{}", path.display(), pattern)
    };
    Ok(glob::glob(&pattern)?)
}
