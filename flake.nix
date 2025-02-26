# SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
#
# SPDX-License-Identifier: EUPL-1.2

{
  outputs = _: {
    overlays.default = import ./overlay.nix;
  };
}
