# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using autoPatchelfVenvShellHook"
if [[ -z "${venvDir-}" ]]; then
    echo "Error: \`venvDir\` should be set when using \`autoPatchelfVenvShellHook\`."
    exit 1
fi

# shellcheck disable=SC1091
source @autoPatchelfHook@

_autoPatchelfVenvChecksum() {
    @nix@/bin/nix-hash --type sha256 "$venvDir/bin" "$venvDir/@pythonSitePackages@"
}

export UV_LINK_MODE=copy

autoPatchelfVenvShellHook() {
    local VENV_CHECKSUM_FILE="$venvDir/venv.checksum"
    local EXPECTED_VENV_CHECKSUM=

    if [[ -f "$VENV_CHECKSUM_FILE" ]]; then
        EXPECTED_VENV_CHECKSUM=$(<"$VENV_CHECKSUM_FILE")
    fi

    if [[ "$(_autoPatchelfVenvChecksum)" != "$EXPECTED_VENV_CHECKSUM" ]]; then
        for p in "${libraries[@]-}"; do
            addAutoPatchelfSearchPath "$p"
        done
        autoPatchelf "$venvDir/bin" "$venvDir/lib" |
            grep -v "searching for dependencies"

        local rc="${PIPESTATUS[0]}"
        [[ "$rc" -ne 0 ]] && {
            echo "Add missing dependencies to \`libraries\`"
            exit "$rc"
        }

        # patchelf may change the checksum
        _autoPatchelfVenvChecksum > "$VENV_CHECKSUM_FILE"
    fi
}

shellHook="${shellHook-}"$'\nautoPatchelfVenvShellHook'
