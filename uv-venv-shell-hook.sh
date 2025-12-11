# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

# shellcheck shell=bash

echo "Using uvVenvShellHook"
venvDir=.venv

export UV_PYTHON_PREFERENCE=only-system

uvVenvShellHook() {
    local UV_INPUTS_FILE="$venvDir/uv.inputs"
    local EXPECTED_UV_INPUTS=

    if [[ -f "$UV_INPUTS_FILE" ]]; then
        EXPECTED_UV_INPUTS=$(<"$UV_INPUTS_FILE")
    fi

    declare -a uvExtraArgsArray
    concatTo uvExtraArgsArray uvExtraArgs

    declare -a uvOverrideCflagsArray
    declare -a uvOverrideLdflagsArray

    if [[ -n ${uvOverrideCc} ]]; then
        declare -x CC="${uvOverrideCc}"
        uvOverridecflagsArray+=("$NIX_CFLAGS_COMPILE")
        uvOverrideLdflagsArray+=("$NIX_LDFLAGS_COMPILE")
    fi

    concatTo uvOverrideCflagsArray uvOverrideCflags
    concatTo uvOverrideLdflagsArray uvOverrideLdflags

    [[ ${#uvOverrideCflagsArray[@]} -gt 0 ]] && declare -x CFLAGS="${uvOverrideCflagsArray[*]}"
    [[ ${#uvOverrideLdflagsArray[@]} -gt 0 ]] && declare -x LDFLAGS="${uvOverrideLdflagsArray[*]}"

    local ACTUAL_UV_INPUTS
    ACTUAL_UV_INPUTS="$(@nix@/bin/nix-hash --type sha256 "$venvDir/../uv.lock"):${uvExtraArgsArray[*]}"

    if [[ "$ACTUAL_UV_INPUTS" != "$EXPECTED_UV_INPUTS" ]]; then

        NIX_ENFORCE_PURITY=0 uv venv --allow-existing
        NIX_ENFORCE_PURITY=0 \
            PATH=${PATH}:@git@/bin \
            uv sync --frozen "${uvExtraArgsArray[@]}" || exit $?
        echo "$ACTUAL_UV_INPUTS" >"$UV_INPUTS_FILE"
    fi

    # shellcheck disable=SC1091
    source "$venvDir/bin/activate"
}

shellHook="${shellHook-}"$'\nuvVenvShellHook'
