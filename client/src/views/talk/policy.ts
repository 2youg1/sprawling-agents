// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three values of a run policy beside the mode, as one control on
// the composer's settings row (refrain roadmap 4-2): its face is the
// write limit, and its menu holds the write limit, the admission
// requirement and the landing as three columns, because most
// dispatches change none of the last two. The values and their order
// are the wire's (`core/commands.ts`); this file only names them and
// says what a pick does.

import { ADMISSIONS, FIRST_POLICY, LANDINGS, WRITE_LIMITS } from "../../core/commands";
import type { Key, Lang } from "../../core/lang";
import { say } from "../../core/lang";
import type { RunPolicy } from "../../wire";
import type { PopoverColumn } from "../parts/popover";

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
