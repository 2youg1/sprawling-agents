// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Enrolment: the one credential that never becomes a command. It goes
// over HTTP to this page's own origin, because the socket's frame type
// cannot spell a secret; what comes back is the reference to put in
// the attach form, and the value itself is never held past the send.

export type Enrolment =
  | { readonly kind: "stored"; readonly reference: string }
  | { readonly kind: "refused"; readonly reason: string };

export function enrol(
  origin: string,
  realm: string,
  name: string,
  value: string,
): Promise<Enrolment> {
  const reference = `secret:${realm}/${name}`;
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
            reason: reason === "" ? "the city did not say why" : reason,
          })),
    (): Enrolment => ({
      kind: "refused",
      reason: "this browser could not reach the city",
    }),
  );
}

// The realm and name a provider's key is filed under: one key per
// provider, named after it, so the reference reads as what it is.
export function referenceFor(provider: string): { realm: string; name: string } {
  return { realm: "providers", name: provider };
}
