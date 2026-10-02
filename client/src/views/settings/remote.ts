// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the remote group can be showing (client/Spec.lean §4-57), and the one
// sentence each refusal is told in. The group's component drives the
// standing; the gallery draws each one on its own.

import type { Key } from "../../core/lang";
import type { Refusal } from "../../core/remote/connect";
import type { Device } from "../../core/remote/device";
import type { Invitation } from "../../core/remote/invitation";

// What this browser lacks to pair at all: an https:// page, a WebCrypto
// with Ed25519 and X25519, an IndexedDB to keep the key in.
export type Lack = "insecure" | "crypto" | "storage";

export type Standing =
  // Reading what this browser keeps.
  | { readonly kind: "reading" }
  // Nothing kept and no invitation: the group explains the door.
  | { readonly kind: "unpaired" }
  | { readonly kind: "unreadable" }
  | { readonly kind: "lacking"; readonly lack: Lack }
  | { readonly kind: "invited"; readonly invitation: Invitation; readonly told: Told | null; readonly busy: boolean }
  // Paired a moment ago: the seed, once.
  | { readonly kind: "seed"; readonly seed: string; readonly device: Device; readonly persisted: boolean }
  | { readonly kind: "paired"; readonly device: Device; readonly told: Told | null; readonly busy: boolean };

// The last thing a press came back with, told in the group's status line.
export type Told =
  | { readonly kind: "refusal"; readonly refusal: Refusal }
  | { readonly kind: "unkept" }
  | { readonly kind: "locked" };

// The console verbs, spelled as a person types them, beside what each
// does; `docs/operating.md` is where they are written out in full.
export const VERBS: readonly (readonly [string, Key])[] = [
  ["/remote open [--for 12h]", "remote_verb_open"],
  ["/remote pair <name> [--watch]", "remote_verb_pair"],
  ["/remote devices", "remote_verb_devices"],
  ["/remote revoke <name>", "remote_verb_revoke"],
  ["/remote close", "remote_verb_close"],
];

export const LACKS: Readonly<Record<Lack, Key>> = {
  insecure: "remote_lacks_insecure",
  crypto: "remote_lacks_crypto",
  storage: "remote_lacks_storage",
};

// The sentence a press came back with; a code the door gave that has no
// sentence of its own is shown as it came.
export function toldKey(told: Told): { readonly key: Key; readonly code: string | null } {
  switch (told.kind) {
    case "unkept":
      return { key: "remote_unkept", code: null };
    case "locked":
      return { key: "remote_locked", code: null };
    case "refusal":
      return refusalKey(told.refusal);
  }
}

function refusalKey(refusal: Refusal): { readonly key: Key; readonly code: string | null } {
  switch (refusal.kind) {
    case "unreachable":
      return { key: "remote_unreachable", code: null };
    case "closed":
      if (refusal.code === null) return { key: "remote_closed_silent", code: null };
      return refusal.code === "E_GATE_DENIED"
        ? { key: "remote_closed_denied", code: null }
        : { key: "remote_closed_code", code: refusal.code };
    case "refused":
      switch (refusal.why) {
        case "length":
          return { key: "remote_refused_length", code: null };
        case "fingerprint":
          return { key: "remote_refused_fingerprint", code: null };
        case "signature":
          return { key: "remote_refused_signature", code: null };
        case "agreement":
          return { key: "remote_refused_agreement", code: null };
        case "key":
          return { key: "remote_refused_key", code: null };
      }
  }
}

// A string of base32 in groups of four, so a person copies it in pieces.
export function grouped(text: string): string {
  return (text.match(/.{1,4}/g) ?? []).join(" ");
}
