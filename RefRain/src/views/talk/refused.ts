// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { providerClause } from "../../core/provider_failure";
import { toFragment } from "../../core/route";
import { recoveryWords } from "../parts/notice_title";
import type { AxError } from "../../wire";

export interface CardLook {
  readonly role: "alert" | undefined;
  readonly head: string;
  readonly after: string | undefined;
  readonly lines: readonly CardLine[];
  readonly fold: Fold | undefined;
  readonly ways: readonly Way[];
}

export interface CardLine {
  readonly text: string;
  readonly ink: "quiet" | "faint";
}

export interface Fold {
  readonly summary: string;
  readonly text: string;
}

export type Way =
  | { readonly kind: "link"; readonly text: string; readonly href: string }
  | { readonly kind: "press"; readonly label: string; readonly onPress: () => void };

const SETTLED_THERE: ReadonlySet<string> = new Set([
  "E_MODEL_UNCHOSEN",
  "E_CREDENTIAL_MISSING",
  "E_PROVIDER",
  "E_PROVIDER_ACCOUNTS_EXHAUSTED",
  "E_ENDPOINT_DIALECT_UNSUPPORTED",
]);

export function settledBySettings(error: AxError): boolean {
  return SETTLED_THERE.has(error.code) || (error.provider !== undefined && error.provider !== null);
}

function toSettings(lang: Lang): Way {
  return {
    kind: "link",
    text: say(lang, "talk_open_settings"),
    href: toFragment({ kind: "setup" }),
  };
}

export function refusedLook(lang: Lang, error: AxError): CardLook {
  const said = error.recovery === "" ? [] : [error.recovery];
  const provider = error.provider ?? null;
  return {
    role: undefined,
    head: error.code,
    after: `${error.action} · ${error.subject}`,
    lines:
      provider === null
        ? said.map((text) => ({ text, ink: "faint" }))
        : [{ text: providerClause(lang, provider, error.retry), ink: "faint" }],
    fold:
      provider === null || error.recovery === ""
        ? undefined
        : { summary: say(lang, "notices_detail"), text: error.recovery },
    ways: settledBySettings(error) ? [toSettings(lang)] : [],
  };
}

export type Settings = "offered" | "absent" | "decided";

export function failedLook(
  lang: Lang,
  what: string,
  error: AxError | undefined,
  hands: { readonly settings: Settings; readonly onRetry: (() => void) | undefined },
): CardLook {
  const next = error === undefined ? "" : recoveryWords(lang, error.code, error.recovery);
  const settings =
    hands.settings === "offered" ||
    (hands.settings === "decided" && error !== undefined && settledBySettings(error));
  const again = hands.onRetry;
  return {
    role: "alert",
    head: what,
    after: undefined,
    lines:
      error === undefined
        ? []
        : [
            { text: `${error.code} · ${error.subject}`, ink: "quiet" },
            ...(next === "" ? [] : [{ text: next, ink: "faint" } as const]),
          ],
    fold:
      error === undefined || error.recovery === "" || error.recovery === next
        ? undefined
        : { summary: say(lang, "notices_detail"), text: error.recovery },
    ways: [
      ...(settings ? [toSettings(lang)] : []),
      ...(again === undefined
        ? []
        : [{ kind: "press", label: say(lang, "talk_send_again"), onPress: again } as const]),
    ],
  };
}

export function unreadableLook(lang: Lang, at: number, cause: string): CardLook {
  return {
    role: undefined,
    head: fill(say(lang, "talk_unreadable"), { at: String(at) }),
    after: cause,
    lines: [],
    fold: undefined,
    ways: [
      {
        kind: "link",
        text: say(lang, "talk_unreadable_read"),
        href: toFragment({ kind: "record", lens: "ledger" }),
      },
    ],
  };
}
