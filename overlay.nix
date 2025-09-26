# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

final: prev: {
  auto-patchelf-rs = final.callPackage (
    { rustPlatform }:

    rustPlatform.buildRustPackage {
      name = "auto-patchelf";
      src = final.lib.fileset.toSource {
        root = ./.;
        fileset = final.lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./auto-patchelf
        ];
      };
      cargoLock = {
        lockFile = ./Cargo.lock;
      };
      cargoFlags = [
        "--package"
        "auto-patchelf"
      ];
      postPatch = ''
        substituteInPlace auto-patchelf/src/main.rs --replace-fail "@defaultBintools@" "$NIX_BINTOOLS"
      '';
    }
  ) { };
  pythonPackagesExtensions = prev.pythonPackagesExtensions ++ [
    (
      python-final: _python-prev:
      let
        inherit (python-final) callPackage;
      in
      {
        uvVenvShellHook = callPackage (
          {
            makePythonHook,
            nix,
            uv,
          }:

          makePythonHook {
            name = "uv-venv-hook";
            propagatedBuildInputs = [ uv ];
            substitutions = {
              inherit nix;
            };
          } ./uv-venv-shell-hook.sh
        ) { };

        maturin_import_hook = callPackage (
          {
            buildPythonPackage,
            fetchFromGitHub,
            filelock,
            setuptools,
          }:

          let
            version = "0.2.0";
          in
          buildPythonPackage {
            pname = "maturin_import_hook";
            inherit version;
            format = "pyproject";
            dependencies = [ filelock ];
            build-system = [ setuptools ];
            src = fetchFromGitHub {
              owner = "PyO3";
              repo = "maturin-import-hook";
              rev = "v${version}";
              hash = "sha256-5Si5BsuYt8GrOHeyS/Cud5u7BloCYFA/nNsjhVjYQoU=";
            };
          }
        ) { };

        maturinImportShellHook = callPackage (
          {
            makePythonHook,
            maturin,
            maturin_import_hook,
            python,
          }:

          makePythonHook {
            name = "maturin-import-hook";
            propagatedBuildInputs = [
              maturin
              maturin_import_hook
            ];
            substitutions = {
              pythonSitePackages = python.sitePackages;
            };
          } ./maturin-import-shell-hook.sh
        ) { };

        autoPatchelfVenvShellHook = callPackage (
          {
            makePythonHook,
            auto-patchelf-rs,
            bintools,
            nix,
            python,
          }:

          makePythonHook {
            name = "auto-patchelf-venv-hook";
            propagatedBuildInputs = [
              auto-patchelf-rs
              bintools
            ];
            substitutions = {
              inherit nix;
              pythonSitePackages = python.sitePackages;
              autoPatchelfHook = "${final.path}/pkgs/build-support/setup-hooks/auto-patchelf.sh";
            };
          } ./auto-patchelf-venv-shell-hook.sh
        ) { };

        patchVenvShellHook = callPackage (
          {
            makePythonHook,
            python,
          }:
          makePythonHook {
            name = "patch-venv-shell-hook";
            substitutions = {
              inherit (final) patch;
              pythonSitePackages = python.sitePackages;
            };
          } ./patch-venv-shell-hook.sh
        ) { };
      }
    )
  ];
}
