// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { canonical, decode, encode } from "./base32";

export interface Invitation {
  readonly code: string;
  readonly city: Uint8Array<ArrayBuffer>;
}

const CODE_SYMBOLS = 26;
const FINGERPRINT_BYTES = 32;

function fieldsOf(hash: string): URLSearchParams {
  return new URLSearchParams(hash.replace(/^#/, ""));
}

export function isInvitation(hash: string): boolean {
  return fieldsOf(hash).has("pair");
}

export function invitationIn(hash: string): Invitation | null {
  const fields = fieldsOf(hash);
  const code = canonical(fields.get("pair") ?? "");
  const text = (fields.get("city") ?? "").trim().toLowerCase();
  const city = decode(text);
  const codeReads = code.length === CODE_SYMBOLS && decode(code) !== null;
  const cityReads = city !== null && city.length === FINGERPRINT_BYTES && encode(city) === text;
  return codeReads && cityReads ? { code, city } : null;
}
