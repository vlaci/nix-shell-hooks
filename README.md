<!--
SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>

SPDX-License-Identifier: EUPL-1.2
-->

# nix-shell-hooks

This flake provides a few new hook packages when used as an overlay:

- `python3Packages.uvVenvShellHook` to initialize an uv managed virtualenv, similar to `venvShellHook` in nixpkgs.
- `python3Packages.maturinImportShellHook` to inject [`maturin_import_hook`](https://github.com/PyO3/maturin-import-hook) to virtualenv
- `python3Packages.autoPatchelfVenvShellHook` to run `patchelf` to fix missing interpreter and dependencies of installed binaries in virtualenv.
- `python3Packagesr.patchVenvShellHook` to apply patches to the installed virtuaelenv
