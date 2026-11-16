// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { RunBelief } from "./belief";
import type { Key } from "./lang";
import type { View } from "./route";
import type {
  Address,
  BuildingProgress,
  Carry,
  Command,
  Effort,
  RunId,
  RunPolicy,
  Seq,
  SessionTags,
} from "../wire";

export interface Reached {
  readonly run: RunId;
  readonly at: Seq;
}

export function reached(run: RunBelief | undefined): Reached | null {
  return run === undefined ? null : { run: run.run, at: run.lastSeq };
}

export interface Offered {
  readonly endpoint: string;
  readonly model: string;
}

export interface SessionHands {
  readonly tagged: SessionTags | null;
  readonly retag: (next: SessionTags) => boolean;
  readonly whenFrozen: (run: RunId, then: () => void) => void;
  readonly openSession: (room: Address, carry: Carry) => boolean;
  readonly opening: () => boolean;
}

export interface SlashHands extends SessionHands {
  readonly command: (command: Command) => boolean;
  readonly go: (view: View) => void;
  readonly here: Address | null;
  readonly live: Reached | null;
  readonly newest: (room: string) => Reached | null;
  readonly buildings: readonly BuildingProgress[] | undefined;
  readonly models: readonly Offered[];
  readonly effort: Effort | null;
  readonly setEffort: (effort: Effort | null) => void;
  readonly policy: RunPolicy;
  readonly setPolicy: (policy: RunPolicy) => void;
  readonly goal: string;
  readonly write: (line: string) => void;
}

export interface SlashCall {
  readonly verb: string;
  readonly words: readonly string[];
  readonly rest: string;
}

export const SECTIONS = ["actions", "navigation", "sessions"] as const;
export type Section = (typeof SECTIONS)[number];

export interface Slash {
  readonly spelling: string;
  readonly grammar: string;
  readonly about: Key;
  readonly section: Section;
  readonly run: (hands: SlashHands, call: SlashCall) => void;
}
