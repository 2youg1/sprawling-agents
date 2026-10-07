// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the sandbox card's choice of arm is given (client D95:
// `sandbox.svelte` is the seat, `arm_select.look.svelte` draws the
// choice): the arms in the order the card offers them, each named in the
// person's language, and the one held.

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

// The bag for a choice holding `held`; `pick` receives the value a
// person chose, which is always one of `options`.
export function armSelectWire(label: string, held: string, pick: (value: string) => void): ArmSelectWire {
  return {
    "aria-label": label,
    value: held,
    onchange: (event) => {
      pick(event.currentTarget.value);
    },
  };
}
