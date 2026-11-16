// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface ArmOption {
  readonly value: string;
  readonly label: string;
}

export interface ArmSelectWire {
  readonly "aria-label": string;
  readonly value: string;
  readonly onchange: (event: Event & { readonly currentTarget: HTMLSelectElement }) => void;
}

export interface ArmSelectLook {
  readonly options: readonly ArmOption[];
  readonly wire: ArmSelectWire;
}

export function armSelectWire(
  label: string,
  held: string,
  pick: (value: string) => void,
): ArmSelectWire {
  return {
    "aria-label": label,
    value: held,
    onchange: (event) => {
      pick(event.currentTarget.value);
    },
  };
}
