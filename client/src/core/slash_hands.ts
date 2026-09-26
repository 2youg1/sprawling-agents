// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a slash verb is, and every capability it may reach for. The
// verbs themselves are `slash.ts`'s table; a view fills `SlashHands` in
// from what it already has and never names a verb.

import type { RunBelief } from "./belief";
import type { Key } from "./lang";
import type { View } from "./route";
import type { Address, Command, Effort, Mode, RunId, Seq } from "../wire";

// A run a verb can act on: which one, and how far it has got. `/steer`,
// `/stop` and `/diff` need the first, and a branch needs both.
export interface Reached {
  readonly run: RunId;
  readonly at: Seq;
}

// The run a view has in hand, as a verb reaches it, and nothing when
// the view has none. It lives beside the type rather than beside each
// caller: the composer and the palette both fill `SlashHands`, and
// each kept a copy of this one conversion.
export function reached(run: RunBelief | undefined): Reached | null {
  return run === undefined ? null : { run: run.run, at: run.lastSeq };
}

// One model the city could be pointed at, named the way the settings
// page names it: the endpoint that serves it, and its id.
export interface Offered {
  readonly endpoint: string;
  readonly model: string;
}

// Everything a verb may reach for, and nothing a view could not hand
// over from what it already has. A field that is null is a capability
// this place does not have - there is no room to dispatch into on the
// run page - and a verb that needs it does nothing rather than guess.
export interface SlashHands {
  readonly command: (command: Command) => boolean;
  readonly go: (view: View) => void;
  // The room the box speaks to, and the run going in it.
  readonly here: Address | null;
  readonly live: Reached | null;
  // The newest run in a room somebody named by hand.
  readonly newest: (room: string) => Reached | null;
  readonly models: readonly Offered[];
  // How hard a run this verb opens should think, or `null` when the
  // person has chosen no level and the provider decides. It comes from
  // the selector beside the box rather than from the line, because
  // that is where the person set it.
  readonly effort: Effort | null;
  readonly setEffort: (effort: Effort | null) => void;
  // The discipline a run this verb opens works under, from the pill
  // beside the box.
  readonly mode: Mode;
  // What a new run is told to aim at, already in the person's language.
  readonly goal: string;
  // The line in the box: emptied by a verb that ran, refilled by `/help`.
  readonly write: (line: string) => void;
}

// A typed line, cut where the first space is: the verb as spelled, the
// arguments split, and the arguments as typed. `/steer` needs the
// unsplit form, because what follows it is a sentence rather than a
// list of words.
export interface SlashCall {
  readonly verb: string;
  readonly words: readonly string[];
  readonly rest: string;
}

// The palette's groups, in its order. A verb names its own, so a verb
// added or renamed cannot fall into a group nobody chose for it.
export const SECTIONS = ["actions", "navigation", "sessions"] as const;
export type Section = (typeof SECTIONS)[number];

export interface Slash {
  readonly spelling: string;
  // What may follow the spelling, in the notation the help line shows:
  // `<required>`, `[optional]`, `a|b` for a choice.
  readonly grammar: string;
  readonly about: Key;
  readonly section: Section;
  readonly run: (hands: SlashHands, call: SlashCall) => void;
}
