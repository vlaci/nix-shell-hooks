# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using patchVenvShellHook"
if [[ -z "${venvDir-}" ]]; then
    echo "Error: \`venvDir\` should be set when using \`patchVenvShellHook\`."
    exit 1
fi

export UV_LINK_MODE=copy

patchVenvShellHook() {
    local venvPatchesArray=()
    concatTo venvPatchesArray venvPatches

    for p in "${venvPatchesArray[@]}"; do
        if ! 1>/dev/null @patch@/bin/patch -f -R -p1 -s --dry-run -d "$venvDir/@pythonSitePackages@" -i "$p"; then
            echo "applying $p"
            @patch@/bin/patch -f -p1 -d "$venvDir/@pythonSitePackages@" -i "$p"
        fi
    done
}

shellHook="${shellHook-}"$'\npatchVenvShellHook'
