// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The invitation a device scans: `/remote pair` prints a link
// `https://<host>/#pair=<code>&city=<fingerprint>`, and the two values
// ride in the fragment, which a browser never sends to the server, so a
// route that only forwards requests never sees them (`crates/
// remote_access/Spec.lean` §8-6). This module is the one reader of that
// fragment; `core/route.ts` asks it whether the address bar carries one.

import { canonical, decode, encode } from "./base32";

export interface Invitation {
  // The pairing code's canonical text: 26 base32 symbols.
  readonly code: string;
  // The SHA-256 of the city's public key, which the city's answer must match.
  readonly city: Uint8Array<ArrayBuffer>;
}

const CODE_SYMBOLS = 26;
const FINGERPRINT_BYTES = 32;

function fieldsOf(hash: string): URLSearchParams {
  return new URLSearchParams(hash.replace(/^#/, ""));
}

// Whether the fragment is an invitation at all, readable or not: the
// page opens the remote group for both, and says which it is.
export function isInvitation(hash: string): boolean {
  return fieldsOf(hash).has("pair");
}

// The invitation the fragment carries; null when either part is absent
// or is not what the console prints. A fingerprint must be the one
// spelling of its 32 bytes, as `CityFingerprint::read` demands, so two
// fingerprints are equal exactly when their texts are.
export function invitationIn(hash: string): Invitation | null {
  const fields = fieldsOf(hash);
  const code = canonical(fields.get("pair") ?? "");
  const text = (fields.get("city") ?? "").trim().toLowerCase();
  const city = decode(text);
  const codeReads = code.length === CODE_SYMBOLS && decode(code) !== null;
  const cityReads = city !== null && city.length === FINGERPRINT_BYTES && encode(city) === text;
  return codeReads && cityReads ? { code, city } : null;
}
