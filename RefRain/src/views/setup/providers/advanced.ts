// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import type { Proxying, TuningDefaults } from "../../../wire";
import {
  ID_SHAPE,
  PROXYINGS,
  blankLine,
  boxed,
  idOf,
  naming,
  proxyingNote,
  tuningHints,
} from "./draft";
import type { Draft, Line, TuningField } from "./draft";

export type SetDraft = <K extends keyof Draft>(part: K, value: Draft[K]) => void;

export interface Hands {
  readonly setDraft: SetDraft;
  readonly onRenamed: () => void;
}

const TUNING: readonly (readonly [Key, TuningField, number])[] = [
  ["setup_timeout_ms", "timeoutMs", 1000],
  ["setup_request_retries", "requestRetries", 1],
  ["setup_stream_idle", "streamIdleMs", 1000],
];

export interface BoxLook {
  readonly key: string;
  readonly label: string;
  readonly help: string | undefined;
  readonly mono: boolean;
  readonly kind: "text" | "number";
  readonly step: number | undefined;
  readonly pattern: string | undefined;
  readonly value: string;
  readonly placeholder: string;
  readonly input: (value: string) => void;
}

export interface CellLook {
  readonly label: string;
  readonly value: string;
  readonly input: (value: string) => void;
}

export interface PairRowLook {
  readonly key: string;
  readonly name: CellLook;
  readonly value: CellLook;
  readonly remove: { readonly label: string; readonly press: () => void };
}

export interface PairTableLook {
  readonly key: "headers" | "overrides";
  readonly title: string;
  readonly rows: readonly PairRowLook[];
  readonly add: { readonly label: string; readonly press: () => void };
}

export interface AdvancedLook {
  readonly title: string;
  readonly naming: readonly BoxLook[];
  readonly tuning: readonly BoxLook[];
  readonly proxying: {
    readonly label: string;
    readonly options: readonly { readonly value: Proxying; readonly label: string }[];
    readonly held: Proxying;
    readonly pick: (rule: Proxying) => void;
    readonly help: string;
    readonly note: string | undefined;
  };
  readonly tables: readonly PairTableLook[];
}

export function retyped(
  rows: readonly Line[],
  key: string,
  part: "name" | "value",
  text: string,
): Line[] {
  return rows.map((each) => (each.key === key ? { ...each, [part]: text } : each));
}

export function lookOf(
  draft: Draft,
  defaults: TuningDefaults | undefined,
  lang: Lang,
  hands: Hands,
): AdvancedLook {
  const hints = tuningHints(draft, defaults, say(lang, "setup_retries_until_halted"));
  const ruleNote = proxyingNote(draft.proxying);
  const rename = (part: "id" | "label", typed: string): void => {
    hands.setDraft(part, naming(typed));
    hands.onRenamed();
  };
  return {
    title: say(lang, "setup_advanced"),
    naming: [
      {
        key: "id",
        label: say(lang, "setup_id"),
        help: say(lang, "setup_id_help"),
        mono: true,
        kind: "text",
        step: undefined,
        pattern: ID_SHAPE.source,
        value: boxed(draft.id),
        placeholder: idOf(draft),
        input: (value) => {
          rename("id", value);
        },
      },
      {
        key: "label",
        label: say(lang, "setup_display_name"),
        help: say(lang, "setup_display_name_help"),
        mono: false,
        kind: "text",
        step: undefined,
        pattern: undefined,
        value: boxed(draft.label),
        placeholder: idOf(draft),
        input: (value) => {
          rename("label", value);
        },
      },
    ],
    tuning: TUNING.map(([word, part, step]) => ({
      key: part,
      label: say(lang, word),
      help: undefined,
      mono: true,
      kind: "number",
      step,
      pattern: undefined,
      value: draft[part],
      placeholder: hints[part],
      input: (value) => {
        hands.setDraft(part, value);
      },
    })),
    proxying: {
      label: say(lang, "setup_proxying"),
      options: PROXYINGS.map(([setting, word]) => ({ value: setting, label: say(lang, word) })),
      held: draft.proxying,
      pick: (rule) => {
        hands.setDraft("proxying", rule);
      },
      help: say(lang, "setup_proxying_help"),
      note: ruleNote === undefined ? undefined : say(lang, ruleNote),
    },
    tables: [
      pairTable("headers", draft.headers, lang, hands, [
        "setup_headers",
        "setup_header_name",
        "setup_header_value",
      ]),
      pairTable("overrides", draft.overrides, lang, hands, [
        "setup_overrides",
        "setup_pointer",
        "setup_override_value",
      ]),
    ],
  };
}

function pairTable(
  part: "headers" | "overrides",
  rows: readonly Line[],
  lang: Lang,
  hands: Hands,
  [title, name, value]: readonly [Key, Key, Key],
): PairTableLook {
  const change = (next: Line[]): void => {
    hands.setDraft(part, next);
  };
  return {
    key: part,
    title: say(lang, title),
    rows: rows.map((row) => ({
      key: row.key,
      name: {
        label: say(lang, name),
        value: row.name,
        input: (text) => {
          change(retyped(rows, row.key, "name", text));
        },
      },
      value: {
        label: say(lang, value),
        value: row.value,
        input: (text) => {
          change(retyped(rows, row.key, "value", text));
        },
      },
      remove: {
        label: say(lang, "setup_remove_row"),
        press: () => {
          change(rows.filter((each) => each.key !== row.key));
        },
      },
    })),
    add: {
      label: say(lang, "setup_add_row"),
      press: () => {
        change([...rows, blankLine()]);
      },
    },
  };
}
