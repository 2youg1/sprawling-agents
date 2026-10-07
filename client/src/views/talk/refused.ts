// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a refusal is said in the thread, as the card its look draws
// (`refusal_card.look.svelte`): a refusal a turn came to, a message that
// got no reply, and a record that did not read back are three readings
// of one card - a first line in the alert ink, the lines under it, the
// city's own sentence one fold away, and the ways out.
//
// Whether the way past a refusal goes through the model settings: no
// model chosen, a key missing, or a provider that turned the call down.
// One answer, so the refusal inside a turn and the one above the box
// offer the same next step.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { providerClause } from "../../core/provider_failure";
import { toFragment } from "../../core/route";
import { recoveryWords } from "../parts/notice_title";
import type { AxError } from "../../wire";

export interface CardLook {
  // `alert` where the card is news a screen reader should say at once:
  // words the person sent that got no reply.
  readonly role: "alert" | undefined;
  // The first line: what happened in the alert ink, then the rest of
  // the line after a separator in the card's quiet ink.
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

// The city's own sentence, which names the case in front of it where
// the page's words speak for the whole code, kept one fold away.
export interface Fold {
  readonly summary: string;
  readonly text: string;
}

// A way out of the card: a place to go, or an action to take again.
export type Way =
  | { readonly kind: "link"; readonly text: string; readonly href: string }
  | { readonly kind: "press"; readonly label: string; readonly onPress: () => void };

// The codes whose way out is the settings page.
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
  return { kind: "link", text: say(lang, "talk_open_settings"), href: toFragment({ kind: "setup" }) };
}

// One refusal a turn came to. A failed model call names its kind, said
// from `lang.json` in the reader's language, and the city's sentence
// stays folded beneath it for whoever is debugging; any other refusal
// has only the city's sentence, shown as it is.
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
    fold: provider === null || error.recovery === "" ? undefined : { summary: say(lang, "notices_detail"), text: error.recovery },
    ways: settledBySettings(error) ? [toSettings(lang)] : [],
  };
}

// Whether a message that got no reply offers the model settings: a
// caller that knows says so, and otherwise the refusal decides.
export type Settings = "offered" | "absent" | "decided";

// A message that got no reply, drawn where the reply would have been:
// what happened, the code and its subject, the next step in the
// reader's words, the city's sentence folded where it says more, and
// the ways out - the model settings, and sending the words again.
// Nothing a person sent may end in silence.
export function failedLook(
  lang: Lang,
  what: string,
  error: AxError | undefined,
  hands: { readonly settings: Settings; readonly onRetry: (() => void) | undefined },
): CardLook {
  const next = error === undefined ? "" : recoveryWords(lang, error.code, error.recovery);
  const settings =
    hands.settings === "offered" || (hands.settings === "decided" && error !== undefined && settledBySettings(error));
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
      ...(again === undefined ? [] : [{ kind: "press", label: say(lang, "talk_send_again"), onPress: again } as const]),
    ],
  };
}

// A record that did not read back stays in the turn with what stopped
// the reading, and leads to the Ledger, where the record itself can
// still be read.
export function unreadableLook(lang: Lang, at: number, cause: string): CardLook {
  return {
    role: undefined,
    head: fill(say(lang, "talk_unreadable"), { at: String(at) }),
    after: cause,
    lines: [],
    fold: undefined,
    ways: [{ kind: "link", text: say(lang, "talk_unreadable_read"), href: toFragment({ kind: "record", lens: "ledger" }) }],
  };
}
