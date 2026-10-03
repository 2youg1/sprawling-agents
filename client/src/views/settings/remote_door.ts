// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The door controls of the remote group: open the door for a chosen
// time, replace the city key, and the code input both of them answer
// with (client/spec/Views/Door.lean is the model; `step` is it, event for
// event). A press asks the city; the city prints a one-time code on its
// own console and answers `E_APPROVAL_PENDING`; the code typed here
// goes back in `ConfirmRemoteDoor` (`crates/remote_access/Spec.lean` D4).
// Closing needs no code and does not pass through this machine.

import { closeRemoteDoor, confirmRemoteDoor, openRemoteDoor, replaceCityKey } from "../../core/commands";
import type { Key } from "../../core/lang";
import type { AxCode, Command } from "../../wire";

export type Opener = "door" | "key";

export type Phase =
  | { readonly kind: "idle" }
  | { readonly kind: "requesting"; readonly opener: Opener }
  | { readonly kind: "awaiting"; readonly opener: Opener }
  | { readonly kind: "confirming"; readonly opener: Opener }
  | { readonly kind: "refused"; readonly opener: Opener; readonly code: AxCode };

export type Focus = { readonly kind: "elsewhere" } | { readonly kind: "opener"; readonly opener: Opener } | { readonly kind: "input" };

export interface Door {
  readonly phase: Phase;
  readonly typed: string;
  readonly focus: Focus;
}

export const IDLE: Door = { phase: { kind: "idle" }, typed: "", focus: { kind: "elsewhere" } };

export type DoorSent = { readonly kind: "open" } | { readonly kind: "replace" } | { readonly kind: "confirm"; readonly code: string };

export type DoorEvent =
  | { readonly kind: "press"; readonly opener: Opener }
  | { readonly kind: "pending" }
  | { readonly kind: "refusal"; readonly code: AxCode }
  | { readonly kind: "unsent" }
  | { readonly kind: "settled" }
  | { readonly kind: "type"; readonly text: string }
  | { readonly kind: "submit" }
  | { readonly kind: "escape" };

type Stepped = readonly [Door, DoorSent | null];

// One event, the door it leaves and the one command it sends, if any.
// Only while a request or a confirmation is in flight is a refusal this
// machine's answer: the page holds one refusal for every command, so a
// refusal that arrives at any other time belongs to some other control.
export function step(door: Door, event: DoorEvent): Stepped {
  const phase = door.phase;
  const same: Stepped = [door, null];
  switch (event.kind) {
    case "press":
      return phase.kind === "idle" || phase.kind === "refused"
        ? [{ phase: { kind: "requesting", opener: event.opener }, typed: "", focus: { kind: "opener", opener: event.opener } }, asked(event.opener)]
        : same;
    case "pending":
      return phase.kind === "requesting" ? [{ phase: { kind: "awaiting", opener: phase.opener }, typed: "", focus: { kind: "input" } }, null] : same;
    case "type":
      return phase.kind === "awaiting" ? [{ ...door, typed: event.text }, null] : same;
    case "submit":
      return phase.kind === "awaiting" && door.typed !== ""
        ? [{ phase: { kind: "confirming", opener: phase.opener }, typed: door.typed, focus: { kind: "input" } }, { kind: "confirm", code: door.typed }]
        : same;
    case "escape":
      return phase.kind === "awaiting" ? [{ phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } }, null] : same;
    case "refusal":
      return phase.kind === "requesting" || phase.kind === "confirming"
        ? [{ phase: { kind: "refused", opener: phase.opener, code: event.code }, typed: "", focus: { kind: "opener", opener: phase.opener } }, null]
        : same;
    case "unsent":
      // The command never left the page: the city said nothing, so the
      // machine goes back to rest rather than inventing a refusal code.
      return phase.kind === "requesting" || phase.kind === "confirming"
        ? [{ phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } }, null]
        : same;
    case "settled":
      return phase.kind === "confirming" ? [{ phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } }, null] : same;
  }
}

function asked(opener: Opener): DoorSent {
  switch (opener) {
    case "door":
      return { kind: "open" };
    case "key":
      return { kind: "replace" };
  }
}

// What the city's answer to a press or a code is, to this machine.
export function answerOf(code: AxCode): DoorEvent {
  return code === "E_APPROVAL_PENDING" ? { kind: "pending" } : { kind: "refusal", code };
}

// The sentence a refused press is told in: a wrong or expired code asks
// for a new press, a city with no console says where to run it, any
// other code is shown as it came.
const REFUSED: Partial<Readonly<Record<AxCode, Key>>> = {
  E_GATE_DENIED: "remote_door_denied",
  E_TOOL_UNAVAILABLE: "remote_door_no_console",
  E_WIRE_MISMATCH: "remote_door_unknown",
};

export function refusedKey(code: AxCode): Key {
  return REFUSED[code] ?? "remote_door_refused";
}

// How long the door may stay open, as `/remote open --for` spells it; at
// most a week, which the console's parser also holds. Twelve hours is
// the console's own example.
const MINUTE_MS = 60_000;
export const LASTINGS: readonly (readonly [string, number])[] = [
  ["30m", 30 * MINUTE_MS],
  ["2h", 120 * MINUTE_MS],
  ["12h", 720 * MINUTE_MS],
  ["2d", 2880 * MINUTE_MS],
  ["7d", 10080 * MINUTE_MS],
];
export const LASTING_DEFAULT = "12h";

// The command a sent event or the close control puts on the wire.
export function doorCommand(sent: DoorSent | "close", lastingMs: number): Command {
  if (sent === "close") return closeRemoteDoor();
  switch (sent.kind) {
    case "open":
      return openRemoteDoor(lastingMs);
    case "replace":
      return replaceCityKey();
    case "confirm":
      return confirmRemoteDoor(sent.code);
  }
}
