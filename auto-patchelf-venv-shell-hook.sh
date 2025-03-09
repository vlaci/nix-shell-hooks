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

export UV_LINK_MODE=copy

autoPatchelfVenvShellHook() {
    for p in "${libraries[@]-}"; do
        addAutoPatchelfSearchPath "$p"
    done
    autoPatchelf "$venvDir/bin" "$venvDir/lib"
}

shellHook="${shellHook-}"$'\nautoPatchelfVenvShellHook'
