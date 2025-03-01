# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using uvVenvShellHook"
venvDir=.venv

export UV_PYTHON_PREFERENCE=only-system

uvVenvShellHook() {
    local UV_LOCK_CHECKSUM_FILE="$venvDir/uv.lock.checksum"
    local EXPECTED_UV_LOCK_CHECKSUM=

    if [[ -f "$UV_LOCK_CHECKSUM_FILE" ]]; then
        EXPECTED_UV_LOCK_CHECKSUM=$(<"$UV_LOCK_CHECKSUM_FILE")
    fi

    local ACTUAL_UV_LOCK_CHECKSUM
    ACTUAL_UV_LOCK_CHECKSUM=$(@nix@/bin/nix-hash --type sha256 "$venvDir/../uv.lock")

    if [[ "$ACTUAL_UV_LOCK_CHECKSUM" != "$EXPECTED_UV_LOCK_CHECKSUM" ]]; then
        local uvExtraArgsArray=()
        concatTo uvExtraArgsArray uvExtraArgs

        NIX_ENFORCE_PURITY=0 uv sync --frozen "${uvExtraArgsArray[@]}" || exit $?
        echo "$ACTUAL_UV_LOCK_CHECKSUM" > "$UV_LOCK_CHECKSUM_FILE"
    fi

    # shellcheck disable=SC1091
    source "$venvDir/bin/activate"
}

shellHook="${shellHook-}"$'\nuvVenvShellHook'
