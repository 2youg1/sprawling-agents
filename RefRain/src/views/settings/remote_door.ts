// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  closeRemoteDoor,
  confirmRemoteDoor,
  openRemoteDoor,
  replaceCityKey,
} from "../../core/commands";
import type { CommandReceipt } from "../../core/socket";
import type { Key } from "../../core/lang";
import type { AxCode, Command, IdemKey } from "../../wire";
import type { GlyphName } from "../parts/glyph";

export type Opener = "door" | "key";

export type Phase =
  | { readonly kind: "idle" }
  | { readonly kind: "requesting"; readonly opener: Opener }
  | { readonly kind: "awaiting"; readonly opener: Opener }
  | { readonly kind: "confirming"; readonly opener: Opener }
  | {
      readonly kind: "unknown";
      readonly opener: Opener;
      readonly during: "requesting" | "confirming";
    }
  | { readonly kind: "refused"; readonly opener: Opener; readonly code: AxCode };

export type Focus =
  | { readonly kind: "elsewhere" }
  | { readonly kind: "opener"; readonly opener: Opener }
  | { readonly kind: "input" };

export interface Door {
  readonly phase: Phase;
  readonly typed: string;
  readonly focus: Focus;
}

export const IDLE: Door = { phase: { kind: "idle" }, typed: "", focus: { kind: "elsewhere" } };

export type DoorSent =
  | { readonly kind: "open" }
  | { readonly kind: "replace" }
  | { readonly kind: "confirm"; readonly code: string };

export type DoorEvent =
  | { readonly kind: "press"; readonly opener: Opener }
  | { readonly kind: "pending" }
  | { readonly kind: "refusal"; readonly code: AxCode }
  | { readonly kind: "unsent" }
  | { readonly kind: "settled" }
  | { readonly kind: "timed_out" }
  | { readonly kind: "checked" }
  | { readonly kind: "type"; readonly text: string }
  | { readonly kind: "submit" }
  | { readonly kind: "escape" };

type Stepped = readonly [Door, DoorSent | null];

export function step(door: Door, event: DoorEvent): Stepped {
  const phase = door.phase;
  const same: Stepped = [door, null];
  switch (event.kind) {
    case "press":
      return phase.kind === "idle" || phase.kind === "refused"
        ? [
            {
              phase: { kind: "requesting", opener: event.opener },
              typed: "",
              focus: { kind: "opener", opener: event.opener },
            },
            asked(event.opener),
          ]
        : same;
    case "pending":
      return phase.kind === "requesting" ||
        (phase.kind === "unknown" && phase.during === "requesting")
        ? [
            {
              phase: { kind: "awaiting", opener: phase.opener },
              typed: "",
              focus: { kind: "input" },
            },
            null,
          ]
        : same;
    case "type":
      return phase.kind === "awaiting" ? [{ ...door, typed: event.text }, null] : same;
    case "submit":
      return phase.kind === "awaiting" && door.typed !== ""
        ? [
            {
              phase: { kind: "confirming", opener: phase.opener },
              typed: door.typed,
              focus: { kind: "input" },
            },
            { kind: "confirm", code: door.typed },
          ]
        : same;
    case "escape":
      return phase.kind === "awaiting"
        ? [
            { phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } },
            null,
          ]
        : same;
    case "refusal":
      return phase.kind === "requesting" || phase.kind === "confirming" || phase.kind === "unknown"
        ? [
            {
              phase: { kind: "refused", opener: phase.opener, code: event.code },
              typed: "",
              focus: { kind: "opener", opener: phase.opener },
            },
            null,
          ]
        : same;
    case "unsent":
      return phase.kind === "requesting" || phase.kind === "confirming"
        ? [
            { phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } },
            null,
          ]
        : same;
    case "timed_out":
      return phase.kind === "requesting" || phase.kind === "confirming"
        ? [
            {
              phase: { kind: "unknown", opener: phase.opener, during: phase.kind },
              typed: "",
              focus: { kind: "opener", opener: phase.opener },
            },
            null,
          ]
        : same;
    case "checked":
      return phase.kind === "unknown"
        ? [
            { phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } },
            null,
          ]
        : same;
    case "settled":
      return phase.kind === "confirming" ||
        (phase.kind === "unknown" && phase.during === "confirming")
        ? [
            { phase: { kind: "idle" }, typed: "", focus: { kind: "opener", opener: phase.opener } },
            null,
          ]
        : same;
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

export function answerOf(code: AxCode): DoorEvent {
  return code === "E_APPROVAL_PENDING" ? { kind: "pending" } : { kind: "refusal", code };
}

const REFUSED: Partial<Readonly<Record<AxCode, Key>>> = {
  E_GATE_DENIED: "remote_door_denied",
  E_TOOL_UNAVAILABLE: "remote_door_no_console",
  E_WIRE_MISMATCH: "remote_door_unknown",
};

export function refusedKey(code: AxCode): Key {
  return REFUSED[code] ?? "remote_door_refused";
}

const MINUTE_MS = 60_000;
export const LASTINGS: readonly (readonly [string, number])[] = [
  ["30m", 30 * MINUTE_MS],
  ["2h", 120 * MINUTE_MS],
  ["12h", 720 * MINUTE_MS],
  ["2d", 2880 * MINUTE_MS],
  ["7d", 10080 * MINUTE_MS],
];
export const LASTING_DEFAULT = "12h";

export function doorCommand(
  sent: DoorSent | "close",
  lastingMs: number,
  approval: IdemKey | null = null,
): Command | null {
  if (sent === "close") return closeRemoteDoor();
  switch (sent.kind) {
    case "open":
      return openRemoteDoor(lastingMs);
    case "replace":
      return replaceCityKey();
    case "confirm":
      return approval === null ? null : confirmRemoteDoor(sent.code, approval);
  }
}

export function requestOf(command: Command): IdemKey | null {
  if ("open_remote_door" in command) return command.open_remote_door.idem;
  if ("replace_city_key" in command) return command.replace_city_key.idem;
  if ("confirm_remote_door" in command) return command.confirm_remote_door.idem;
  if ("close_remote_door" in command) return command.close_remote_door.idem;
  return null;
}

export function receiptOf(
  request: IdemKey | null,
  receipts: ReadonlyMap<IdemKey, CommandReceipt>,
): DoorEvent | null {
  const receipt = request === null ? undefined : receipts.get(request);
  if (receipt === undefined) return null;
  return receipt.kind === "accepted" ? { kind: "settled" } : answerOf(receipt.error.code);
}

export interface DoorKeyLook {
  readonly glyph: GlyphName;
  readonly label: string;
  readonly note: string;
  readonly wire: DoorKeyWire;
}

export interface DoorKeyWire {
  readonly type: "button" | "submit";
  readonly "aria-disabled": boolean;
  readonly onclick?: () => void;
}
