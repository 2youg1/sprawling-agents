// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Enrolment: the one credential that never becomes a command. It goes
// over HTTP to this page's own origin, because the socket's frame type
// cannot spell a secret; what comes back is the reference to put in
// the attach form, and the value itself is never held past the send.

import type { Lang } from "./lang";
import { say } from "./lang";

export type Enrolment =
  | { readonly kind: "stored"; readonly reference: string }
  | { readonly kind: "refused"; readonly reason: string };

// One enrolment, as the caller states it. Five values that mean
// nothing apart: where to send it, what to file it under, the secret
// itself, and the language the answer comes back in.
export interface Enrolling {
  readonly origin: string;
  readonly realm: string;
  readonly name: string;
  readonly value: string;
  readonly lang: Lang;
}

export function enrol(at: Enrolling): Promise<Enrolment> {
  const { origin, realm, name, value, lang } = at;
  const reference = referenceText({ realm, name });
  return fetch(`${origin}/enroll`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ realm, name, value }),
  }).then(
    (response) =>
      response.status === 201
        ? { kind: "stored", reference }
        : response.text().then((reason) => ({
            kind: "refused",
            reason: reason === "" ? say(lang, "enrol_city_said_nothing") : reason,
          })),
    (): Enrolment => ({
      kind: "refused",
      reason: say(lang, "enrol_unreachable"),
    }),
  );
}

// Where one key is filed in the vault.
export interface SecretAt {
  readonly realm: string;
  readonly name: string;
}

// The realm and name a provider's key is filed under: one key per
// provider, named after it, so the reference reads as what it is.
export function referenceFor(provider: string): SecretAt {
  return { realm: "providers", name: provider };
}

// How a place in the vault is spelled in a command.
//
// One spelling, here, because two readers need it and they are on
// opposite sides of an enrolment: the form shows a person what their
// key is about to be filed under, and the command carries the same
// text for `kernel::SecretRef::parse` to read back. A second spelling
// would let the form promise a reference the city cannot resolve.
export function referenceText(at: SecretAt): string {
  return `secret:${at.realm}/${at.name}`;
}

// What this page knows about a key it sent to the vault: the reference
// the city answered with, and the provider id it was filed under.
//
// **The id travels beside the reference because it is what makes the
// reference true.** A reference held alone outlived the id it was
// derived from: renaming the provider left `secret:providers/<old>` in
// the form, and the second provider a person enrolled inherited the
// first one's key. Holding the pair lets every reader ask the only
// question that matters - is this reference the one this id derives -
// and get `null` the moment the name changes.
export interface StoredKey {
  readonly provider: string;
  readonly reference: string;
}

// What the key field is for one provider id right now.
export type KeyField =
  // Nothing is filed under this id: what is typed here is enrolled.
  | { readonly kind: "empty" }
  // A key is filed under this id: leaving the field blank keeps it,
  // typing into it replaces it.
  | { readonly kind: "stored"; readonly reference: string };

export function keyField(held: StoredKey | null, provider: string): KeyField {
  return held !== null && held.provider === provider
    ? { kind: "stored", reference: held.reference }
    : { kind: "empty" };
}

// The reference a command should carry for this provider id: the stored
// one when it was filed under this very id, and nothing otherwise. A
// reference derived from a name nobody enrolled names a key that does
// not exist, which is a 401 a person cannot read off a form.
export function secretFor(held: StoredKey | null, provider: string): string | null {
  const field = keyField(held, provider);
  return field.kind === "stored" ? field.reference : null;
}
