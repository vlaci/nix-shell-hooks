# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using uvVenvShellHook"
venvDir=.venv

export UV_PYTHON_PREFERENCE=only-system

uvVenvShellHook() {
    local uvExtraArgsArray=()
    concatTo uvExtraArgsArray uvExtraArgs

    NIX_ENFORCE_PURITY=0 uv sync --frozen "${uvExtraArgsArray[@]}"

    # shellcheck disable=SC1091
    source "$venvDir/bin/activate"
}

shellHook="${shellHook-}"$'\nuvVenvShellHook'
