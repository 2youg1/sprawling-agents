// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The permissions entry of the settings row (client/Spec.lean §4-60).
// The admission and landing rows are read from the wire's lists; its two
// switches change mode and write in Ui.policy. The entry is a seat and a
// look like the rest of the row (client D95): `permissionsOf` builds the
// value `settings_row.look.svelte` draws, every role, key and focus rule
// inside a wire bag, and the seat (`settings_row.svelte`) holds whether
// the panel is open and where focus goes.

import type { Attachment } from "svelte/attachments";

import { ADMISSIONS, FIRST_POLICY, LANDINGS, WRITE_LIMITS } from "../../core/commands";
import type { Key, Lang } from "../../core/lang";
import { say } from "../../core/lang";
import type { RunPolicy } from "../../wire";
import type { PopoverColumn } from "../parts/popover";
import { HOLD } from "./pill";
import type { FrameWire, KeyPress, TriggerWire } from "./pill";

export const POLICY_SWITCHES = {
  mode: { off: "chat", on: "work" },
  write: { off: "create", on: "full" },
} satisfies {
  readonly mode: { readonly off: RunPolicy["mode"]; readonly on: RunPolicy["mode"] };
  readonly write: { readonly off: RunPolicy["write"]; readonly on: RunPolicy["write"] };
};

type Chosen = "write" | "admit" | "landing";

// The menu: one column per value, each row a value the wire offers, the
// one the page holds marked.
export function policyColumns(lang: Lang, policy: RunPolicy): readonly PopoverColumn[] {
  const column = <K extends Chosen>(field: K, label: Key, values: readonly RunPolicy[K][]): PopoverColumn => ({
    id: field,
    label,
    rows: values.map((each) => ({ id: each, label: say(lang, `admission_value_${each}`), chosen: each === policy[field] })),
  });
  return [
    column("write", "admission_write", WRITE_LIMITS),
    column("admit", "admission_require", ADMISSIONS),
    column("landing", "admission_landing", LANDINGS),
  ];
}

// The policy after one row of the menu is applied; a row no column
// offers changes nothing.
export function picked(policy: RunPolicy, column: string, row: string): RunPolicy {
  const write = WRITE_LIMITS.find((each) => column === "write" && each === row);
  const admit = ADMISSIONS.find((each) => column === "admit" && each === row);
  const landing = LANDINGS.find((each) => column === "landing" && each === row);
  return { ...policy, write: write ?? policy.write, admit: admit ?? policy.admit, landing: landing ?? policy.landing };
}

// What the closed control says: the write limit, then whichever of the
// other two is not its first value, so a create-only experiment is
// never hidden behind a closed menu.
export function policyFace(lang: Lang, policy: RunPolicy): string {
  return [
    say(lang, `admission_value_${policy.write}`),
    policy.admit === FIRST_POLICY.admit ? null : say(lang, `admission_value_${policy.admit}`),
    policy.landing === FIRST_POLICY.landing ? null : say(lang, `admission_value_${policy.landing}`),
  ]
    .filter((each) => each !== null)
    .join(" · ");
}

// The bag spread on one switch, a native checkbox that says it is a
// switch: Space turns it, and it changes its own field alone.
export interface SwitchWire {
  readonly type: "checkbox";
  readonly role: "switch";
  readonly checked: boolean;
  readonly onchange: (event: { readonly currentTarget: { readonly checked: boolean } }) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface SwitchLook {
  readonly key: "mode" | "write";
  readonly label: string;
  readonly wire: SwitchWire;
}

// The bag spread on one native select; the platform owns its keys.
export interface SelectWire {
  readonly value: string | undefined;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface SelectLook {
  readonly key: string;
  readonly label: string;
  readonly options: readonly { readonly key: string; readonly label: string }[];
  readonly wire: SelectWire;
}

// The bag spread on the open panel: a dialog that Escape closes,
// giving focus back to the entry.
export interface PanelWire {
  readonly role: "dialog";
  readonly tabindex: -1;
  readonly "aria-label": string;
  readonly onkeydown: (event: KeyPress) => void;
}

export interface PanelLook {
  readonly wire: PanelWire;
  readonly switches: readonly SwitchLook[];
  readonly selects: readonly SelectLook[];
}

// Everything the look is given for the permissions entry.
export interface PermissionsLook {
  readonly frame: FrameWire;
  readonly trigger: TriggerWire;
  // The mode, then what the closed control says (`policyFace`).
  readonly face: string;
  readonly panel: PanelLook | undefined;
}

// What only the seat can do.
export interface PermissionsHands {
  readonly choose: (policy: RunPolicy) => void;
  readonly toggle: () => void;
  readonly close: (focus: "opener" | "leave") => void;
  readonly inside: (target: EventTarget | null) => boolean;
  readonly hold: {
    readonly frame: Attachment<HTMLElement>;
    readonly trigger: Attachment<HTMLElement>;
    // The first switch, which takes the focus when the panel opens.
    readonly first: Attachment<HTMLElement>;
  };
}

export function permissionsOf(lang: Lang, policy: RunPolicy, open: boolean, hands: PermissionsHands): PermissionsLook {
  const mode = say(lang, `mode_${policy.mode}`);
  const face = `${mode} · ${policyFace(lang, policy)}`;
  return {
    frame: {
      onfocusout: (event) => {
        if (open && !hands.inside(event.relatedTarget)) hands.close("leave");
      },
      [HOLD]: hands.hold.frame,
    },
    trigger: {
      "aria-label": `${say(lang, "talk_permissions")}: ${face}`,
      "aria-haspopup": "dialog",
      "aria-expanded": open,
      onclick: hands.toggle,
      [HOLD]: hands.hold.trigger,
    },
    face,
    panel: open ? panelOf(lang, policy, hands) : undefined,
  };
}

function panelOf(lang: Lang, policy: RunPolicy, hands: PermissionsHands): PanelLook {
  const flip = (field: "mode" | "write", on: boolean): RunPolicy =>
    field === "mode"
      ? { ...policy, mode: on ? POLICY_SWITCHES.mode.on : POLICY_SWITCHES.mode.off }
      : { ...policy, write: on ? POLICY_SWITCHES.write.on : POLICY_SWITCHES.write.off };
  const switchOf = (field: "mode" | "write", label: Key): SwitchLook => ({
    key: field,
    label: say(lang, label),
    wire: {
      type: "checkbox",
      role: "switch",
      checked: policy[field] === POLICY_SWITCHES[field].on,
      onchange: (event) => {
        hands.choose(flip(field, event.currentTarget.checked));
      },
    },
  });
  // The work switch comes first and takes the focus when the panel opens.
  const work = switchOf("mode", "talk_work_switch");
  return {
    wire: {
      role: "dialog",
      tabindex: -1,
      "aria-label": say(lang, "talk_permissions"),
      onkeydown: (event) => {
        if (event.key !== "Escape") return;
        event.preventDefault();
        event.stopPropagation();
        hands.close("opener");
      },
    },
    switches: [{ ...work, wire: { ...work.wire, [HOLD]: hands.hold.first } }, switchOf("write", "talk_write_switch")],
    selects: policyColumns(lang, policy)
      .filter((column) => column.id !== "write")
      .map((column) => ({
        key: column.id,
        label: say(lang, column.label),
        options: column.rows.map((row) => ({ key: row.id, label: row.label })),
        wire: {
          value: column.rows.find((row) => row.chosen === true)?.id,
          onchange: (event) => {
            hands.choose(picked(policy, column.id, event.currentTarget.value));
          },
        },
      })),
  };
}
