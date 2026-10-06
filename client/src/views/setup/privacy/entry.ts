// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One privacy entry from the press to the result (client/spec/Views/Privacy.lean):
// the state machine each control's entry runs, and the props an entry's
// look is drawn from. The machine is `step` in the model, transcribed;
// `entry.test.ts` replays the model's trace vectors through it. An
// idem is whatever the caller mints, compared by identity, so a result
// for another page's operation never moves this entry.

import type { Weight } from "../../parts/glyph";

export type Action = "apply" | "restore" | "reconcile";

// The three results the page tells apart.
export type Result = "running" | "done" | "refused";

export type Stage<I> =
  | { readonly kind: "idle" }
  | { readonly kind: "confirming"; readonly action: Action }
  | { readonly kind: "sending"; readonly action: Action; readonly idem: I }
  | { readonly kind: "unsent"; readonly action: Action }
  | { readonly kind: "settled"; readonly idem: I }
  | { readonly kind: "refused"; readonly idem: I };

export type Event<I> =
  | { readonly kind: "press"; readonly action: Action }
  | { readonly kind: "cancel" }
  | { readonly kind: "confirm"; readonly idem: I }
  | { readonly kind: "lost"; readonly idem: I }
  | { readonly kind: "outcome"; readonly idem: I; readonly result: Result };

export interface Send<I> {
  readonly action: Action;
  readonly idem: I;
}

export interface Step<I> {
  readonly stage: Stage<I>;
  readonly send: Send<I> | null;
}

export const IDLE = { kind: "idle" } as const;

// Whether a press may open the confirmation: no dialog is open for this
// entry and no command of it is in flight.
export function pressable<I>(stage: Stage<I>): boolean {
  switch (stage.kind) {
    case "idle":
    case "unsent":
    case "settled":
    case "refused":
      return true;
    case "confirming":
    case "sending":
      return false;
  }
}

export function step<I>(offered: (action: Action) => boolean, stage: Stage<I>, event: Event<I>): Step<I> {
  const still: Step<I> = { stage, send: null };
  switch (event.kind) {
    case "press":
      return pressable(stage) && offered(event.action)
        ? { stage: { kind: "confirming", action: event.action }, send: null }
        : still;
    case "cancel":
      return stage.kind === "confirming" ? { stage: IDLE, send: null } : still;
    case "confirm":
      return stage.kind === "confirming"
        ? {
            stage: { kind: "sending", action: stage.action, idem: event.idem },
            send: { action: stage.action, idem: event.idem },
          }
        : still;
    case "lost":
      return stage.kind === "sending" && stage.idem === event.idem
        ? { stage: { kind: "unsent", action: stage.action }, send: null }
        : still;
    case "outcome":
      if (stage.kind !== "sending" || stage.idem !== event.idem) return still;
      switch (event.result) {
        case "running":
          return still;
        case "done":
          return { stage: { kind: "settled", idem: event.idem }, send: null };
        case "refused":
          return { stage: { kind: "refused", idem: event.idem }, send: null };
      }
  }
}

// One labelled reading on an entry: the current value, the value
// written, the original, the scope.
export interface Fact {
  readonly label: string;
  readonly value: string;
  // A registry value, a path or a number, set in the figure face.
  readonly figure: boolean;
}

export interface Press {
  readonly label: string;
  readonly onPress: () => void;
}

// A labelled sentence: what an entry does, what it keeps private, what
// it affects, what a person could overlook.
export interface Note {
  readonly label: string;
  readonly text: string;
}

// Everything an entry's look draws, already in the person's language.
// The look holds no wiring: it calls `onPress` and nothing else, so a
// look from a UI library that takes these props keeps the entry's
// behaviour (the front-end ruling: wiring and state apart from the
// appearance).
export interface EntryLook {
  // The element id the not-written lines' links scroll to.
  readonly id: string;
  readonly title: string;
  // The person's own lines this control writes, under their label, or
  // the note that it was added beyond the list.
  readonly lines: Note;
  readonly notes: readonly Note[];
  // Drawn without expanding anything.
  readonly overlooked: Note;
  // The documented editions, the edition note, and how this host's
  // edition reads; `alert` when the documentation says it ignores it.
  readonly editions: Note;
  readonly editionsNote: string;
  readonly fit: string;
  readonly fitWeight: Weight;
  // Set when the sources doubt the entry changes anything today.
  readonly effect: string | null;
  readonly facts: readonly Fact[];
  // The full target, folded under its label.
  readonly target: Note;
  readonly presses: readonly Press[];
  // What became of this page's last operation on the entry.
  readonly status: { readonly text: string; readonly weight: Weight } | null;
}
