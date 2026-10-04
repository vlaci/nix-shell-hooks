# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using maturinImportShellHook"
if [[ -z "${venvDir-}" ]]; then
    echo "Error: \`venvDir\` should be set when using \`maturinImportShellHook\`."
    exit 1
fi

maturinImportShellHook() {
    local siteDir
    siteDir="$(realpath "$venvDir/@pythonSitePackages@")"

    if [[ ! -f "$siteDir/addsite.pth" ]]; then
        "$venvDir"/bin/python -m maturin_import_hook site install --detect-uv
    fi
    echo 'import sys; exec(open("'"$siteDir"'/sitecustomize.py").read())' > "$siteDir/addsite.pth"
}

shellHook="${shellHook-}"$'\nmaturinImportShellHook'
