// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Weight } from "../../parts/glyph";

export type Action = "apply" | "restore" | "reconcile";

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

export function step<I>(
  offered: (action: Action) => boolean,
  stage: Stage<I>,
  event: Event<I>,
): Step<I> {
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

export interface Fact {
  readonly label: string;
  readonly value: string;

  readonly figure: boolean;
}

export interface Press {
  readonly label: string;
  readonly onPress: () => void;
}

export interface Note {
  readonly label: string;
  readonly text: string;
}

export interface EntryLook {
  readonly id: string;
  readonly title: string;

  readonly lines: Note;
  readonly notes: readonly Note[];

  readonly overlooked: Note;

  readonly editions: Note;
  readonly editionsNote: string;
  readonly fit: string;
  readonly fitWeight: Weight;

  readonly effect: string | null;
  readonly facts: readonly Fact[];

  readonly target: Note;
  readonly presses: readonly Press[];

  readonly status: { readonly text: string; readonly weight: Weight } | null;
}
