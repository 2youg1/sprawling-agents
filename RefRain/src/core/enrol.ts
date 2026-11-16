// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Command } from "../wire";
import { mintIdem } from "./idem";
import type { Lang } from "./lang";
import { say } from "./lang";
import { bearing } from "./socket";

export type Enrolment =
  | { readonly kind: "stored"; readonly reference: string }
  | { readonly kind: "refused"; readonly reason: string };

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
  })
    .then((response) =>
      response.text().then((said): Enrolment =>
        response.status === 201
          ? { kind: "stored", reference: said }
          : {
              kind: "refused",
              reason: said === "" ? say(lang, "enrol_city_said_nothing") : said,
            },
      ),
    )
    .catch((): Enrolment => ({
      kind: "refused",
      reason: say(lang, "enrol_unreachable"),
    }));
}

export interface SecretAt {
  readonly realm: string;
  readonly name: string;
}

const PROVIDERS = "providers";
const SEARCH = "search";

export function referenceFor(provider: string, account?: string): SecretAt {
  return {
    realm: PROVIDERS,
    name: account === undefined ? provider : accountName(provider, account),
  };
}

export function searchReferenceFor(supplier: string, account: string): SecretAt {
  return { realm: SEARCH, name: accountName(supplier, account) };
}

function accountName(owner: string, account: string): string {
  return `${owner}.${account}`;
}

export function forgetSecret(reference: string): Command {
  return { forget_secret: { reference, idem: mintIdem() } };
}

export function referenceText(at: SecretAt): string {
  return `secret:${at.realm}/${at.name}`;
}

export interface StoredKey {
  readonly provider: string;
  readonly reference: string;
}

export type KeyField =
  { readonly kind: "empty" } | { readonly kind: "stored"; readonly reference: string };

export function keyField(held: StoredKey | null, provider: string): KeyField {
  return held !== null && held.provider === provider
    ? { kind: "stored", reference: held.reference }
    : { kind: "empty" };
}

export function secretFor(held: StoredKey | null, provider: string): string | null {
  const field = keyField(held, provider);
  return field.kind === "stored" ? field.reference : null;
}
