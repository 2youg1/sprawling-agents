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
import { bearing } from "./socket";

export type Enrolment =
  | { readonly kind: "stored"; readonly reference: string }
  | { readonly kind: "refused"; readonly reason: string };

// One enrolment, as the caller states it. Six values that mean
// nothing apart: where to send it, the pairing code this page was
// opened with, what to file it under, the secret itself, and the
// language the answer comes back in.
export interface Enrolling {
  readonly origin: string;
  readonly token: string | null;
  readonly realm: string;
  readonly name: string;
  readonly value: string;
  readonly lang: Lang;
}

export function enrol(at: Enrolling): Promise<Enrolment> {
  const { origin, token, realm, name, value, lang } = at;
  return fetch(`${origin}/enroll`, {
    method: "POST",
    headers: { "content-type": "application/json", ...bearing(token) },
    body: JSON.stringify({ realm, name, value }),
  }).then(
    (response) =>
      // **The reference comes back from the city, never from here.**
      // `kernel::SecretRef` is the grammar of a vault place and the
      // route answers 201 with the text it parsed, so a realm or a
      // name this page could spell but the city could not resolve is
      // a refusal rather than a reference nothing answers to.
      response.text().then((said): Enrolment =>
        response.status === 201
          ? { kind: "stored", reference: said }
          : {
              kind: "refused",
              reason: said === "" ? say(lang, "enrol_city_said_nothing") : said,
            },
      ),
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

// The realm this page files a provider's key under. `kernel::SecretRef`
// judges the alphabet of a realm and not which words exist, so the word
// is the client's to choose; it is named once because the promise the
// form shows and the command that carries the reference have to mean
// the same place.
const PROVIDERS = "providers";

// The realm and name a provider's key is filed under: one key per
// provider, named after it, so the reference reads as what it is.
export function referenceFor(provider: string): SecretAt {
  return { realm: PROVIDERS, name: provider };
}

// How a place in the vault is spelled, in the one grammar
// `kernel::SecretRef` writes: `secret:`, then a realm and a name of
// `[A-Za-z0-9._-]+` separated by `/`.
//
// This is the page's promise, not the vault's answer: it is what the
// form shows before anything is enrolled. Once the city has answered,
// [`StoredKey.reference`] is what every command carries, because the
// city parses that text back into the reference it stored.
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
