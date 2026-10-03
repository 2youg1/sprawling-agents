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
import type { DoctorCustodyLifetime } from "../../wire";

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

// The console verbs, spelled as a User types them, beside what each
// does; `docs/operating.md` is where they are written out in full. The
// two a pairing starts with are named once, because the steps below
// spell them too.
const OPEN = "/remote open [--for 12h]";
const PAIR = "/remote pair <name> [--watch]";

export const VERBS: readonly (readonly [string, Key])[] = [
  [OPEN, "remote_verb_open"],
  [PAIR, "remote_verb_pair"],
  ["/remote devices", "remote_verb_devices"],
  ["/remote revoke <name>", "remote_verb_revoke"],
  ["/remote close", "remote_verb_close"],
];

// What a pairing looks like from start to end, one numbered step each:
// what the User does at the city's console, what the other device
// opens, and what it shows. The door's verbs are not on the wire
// (`crates/remote_access/Spec.lean` D4), so a step that happens at the
// console carries its spelling and a copy key, never a button.
export interface Step {
  readonly key: Key;
  readonly spelling: string | null;
}

// wording-ok: the shape of the link `/remote pair` prints, a machine
// spelling identical in both languages
const INVITATION = "https://<host>/#pair=<code>&city=<fingerprint>";

export const STEPS: readonly Step[] = [
  { key: "remote_route", spelling: null },
  { key: "remote_step_open", spelling: OPEN },
  { key: "remote_step_pair", spelling: PAIR },
  { key: "remote_step_device", spelling: INVITATION },
  { key: "remote_step_seed", spelling: null },
];

// What happens to the pairing when the city's computer restarts. The
// city key rests in the city's vault, which keeps it as long as the
// vault keeps any credential: Windows Credential Manager and the macOS
// Keychain across restarts, Linux keyutils until the computer reboots
// unless the city uses the encrypted vault file, and the city's own
// memory when no platform store answered. The doctor's `custody` says
// which one this city has; before the doctor has answered, the page
// says all three.
export function restartOf(keeps: DoctorCustodyLifetime | null): Key {
  if (keeps === null) return "remote_restart_unknown";
  switch (keeps) {
    case "across_reboots":
      return "remote_restart_kept";
    case "with_passphrase":
      return "remote_restart_passphrase";
    case "until_reboot":
      return "remote_restart_until_reboot";
    case "this_process":
      return "remote_restart_process";
  }
}

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
