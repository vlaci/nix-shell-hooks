# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      treefmtEval = forAllSystems (
        system:
        inputs.treefmt-nix.lib.evalModule nixpkgs.legacyPackages.${system} (
          { pkgs, ... }:
          {
            projectRootFile = "flake.nix";
            programs = {
              statix.enable = true;
              deadnix.enable = true;
              shellcheck.enable = true;
              nixfmt = {
                enable = true;
                package = pkgs.nixfmt-rfc-style;
              };
            };
          }
        )
      );
    in
    {
      checks = forAllSystems (system: {
        pre-commit = inputs.pre-commit-hooks.lib.${system}.run {
          src = ../.;
          hooks = {
            end-of-file-fixer.enable = true;
            trim-trailing-whitespace.enable = true;
            treefmt = {
              enable = true;
              packageOverrides.treefmt = treefmtEval.${system}.config.build.wrapper;
            };
            reuse = {
              enable = true;
              name = "reuse";
              description = "Run REUSE compliance tests";
              entry = "${nixpkgs.legacyPackages.${system}.reuse}/bin/reuse lint";
              pass_filenames = false;
            };
          };
        };
      });

      devShells = forAllSystems (
        system: with nixpkgs.legacyPackages.${system}; {
          default = mkShell {
            inherit (self.checks.${system}.pre-commit) shellHook;
            buildInputs = self.checks.${system}.pre-commit.enabledPackages ++ [
              cargo
              cargo-flamegraph
              cargo-features-manager
              gdb
              hotspot
              linuxPackages.perf
              samply
              rustc
              rust-analyzer
              rustfmt
              clippy
            ];
          };
        }
      );
    };
}
