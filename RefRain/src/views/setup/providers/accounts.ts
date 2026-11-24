// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../../core/lang";
import type { Key } from "../../../core/lang";
import type { AccountRetries, AxError, KeyState } from "../../../wire";
import {
  cancel,
  edit,
  forget,
  input,
  move,
  pickRetries,
  remove,
  save,
  shown,
  shownRetries,
  unsaveable,
} from "./account_editor";
import type { Editor, Hands, Roster, Step } from "./account_editor";

export type Control = "up" | "down" | "edit" | "remove";

type ElementHold = (element: HTMLElement) => void | (() => void);

export interface ControlLook {
  readonly key: Control;
  readonly label: string;
  readonly why: string | undefined;
  readonly press: () => void;
  readonly hold: ElementHold | undefined;
}

export interface RowLook {
  readonly id: string;
  readonly place: number;
  readonly key: string;
  readonly wanting: boolean;
  readonly controls: readonly ControlLook[];
}

export type FieldName = "id" | "key" | "reference" | "header";

export interface FieldLook {
  readonly name: FieldName;
  readonly label: string;
  readonly value: string;
  readonly secret: boolean;
  readonly mono: boolean;
  readonly disabled: boolean;
  readonly help: string | undefined;
  readonly folded: boolean;
  readonly input: (value: string) => void;
}

export interface RetriesLook {
  readonly label: string;
  readonly options: readonly {
    readonly value: AccountRetries;
    readonly label: string;
    readonly why?: string;
  }[];
  readonly held: AccountRetries | null;
  readonly fallback: string | undefined;
  readonly pick: (retries: AccountRetries) => void;
}

export interface AccountsLook {
  readonly heading: string;
  readonly holdHeading: ElementHold;
  readonly order: string;
  readonly legacy: string | undefined;
  readonly rows: readonly RowLook[];
  readonly retries: RetriesLook | undefined;
  readonly title: string;
  readonly fields: readonly FieldLook[];
  readonly more: string;
  readonly route: string;
  readonly save: {
    readonly label: string;
    readonly why: string | undefined;
    readonly loading: boolean;
    readonly press: () => void;
  };
  readonly cancel: { readonly label: string; readonly press: () => void } | undefined;
  readonly forget:
    { readonly text: string; readonly label: string; readonly press: () => void } | undefined;
  readonly note: string | undefined;
  readonly refused: AxError | undefined;
}

export interface Holds {
  readonly heading: ElementHold;
  readonly control: (id: string, step: Step) => ElementHold;
}

const KEY_WORDS: Record<KeyState, Key> = {
  anonymous: "setup_account_key_anonymous",
  stored: "setup_account_key_stored",
  missing: "setup_account_key_missing",
  environment: "setup_account_key_environment",
  environment_unusable: "setup_account_key_environment_unusable",
  unread: "setup_account_key_unread",
};

const WANTING: Record<KeyState, boolean> = {
  anonymous: false,
  stored: false,
  missing: true,
  environment: false,
  environment_unusable: true,
  unread: false,
};

const RETRY_WORDS: Record<AccountRetries, Key> = {
  one: "setup_account_retries_one",
  two: "setup_account_retries_two",
};

function keyOf(roster: Roster, id: string): KeyState {
  return roster.keys.find((status) => status.id === id)?.key ?? "unread";
}

export function lookOf(editor: Editor, hands: Hands, holds: Holds): AccountsLook {
  const roster = hands.roster();
  const lang = hands.lang();
  const rows = shown(editor, roster);
  const busy =
    editor.pending !== null
      ? say(lang, "setup_account_busy")
      : hands.flight?.current !== null && hands.flight?.current !== undefined
        ? say(lang, "setup_endpoint_busy")
        : undefined;
  const control = (
    key: Control,
    why: Key | null,
    press: () => void,
    hold?: ElementHold,
  ): ControlLook => ({
    key,
    label: say(lang, `setup_account_${key}`),
    why: busy ?? (why === null ? undefined : say(lang, why)),
    press,
    hold,
  });
  const environmental =
    editor.editing !== null && keyOf(roster, editor.editing).startsWith("environment");
  const unsaved = unsaveable(editor, rows);
  const field = (name: FieldName, label: Key, more: Partial<FieldLook> = {}): FieldLook => ({
    name,
    label: say(lang, label),
    value: editor.draft[name],
    secret: false,
    mono: true,
    disabled: busy !== undefined,
    help: undefined,
    folded: false,
    input: (value) => {
      input(editor, name, value);
    },
    ...more,
  });
  const keyHelp = environmental
    ? "setup_account_key_environment_help"
    : editor.editing === null
      ? null
      : "setup_account_key_kept";
  const counted = roster.retries;
  return {
    heading: say(lang, "setup_accounts"),
    holdHeading: holds.heading,
    order: say(
      lang,
      roster.kind === "provider" ? "setup_accounts_order" : "setup_accounts_order_search",
    ),
    legacy: roster.legacy ? say(lang, "setup_accounts_legacy") : undefined,
    rows: rows.map((row, index) => {
      const key = keyOf(roster, row.id);
      return {
        id: row.id,
        place: index + 1,
        key: say(lang, KEY_WORDS[key]),
        wanting: WANTING[key],
        controls: [
          control(
            "up",
            index === 0 ? "setup_account_first" : null,
            () => {
              move(editor, hands, row.id, "up");
            },
            holds.control(row.id, "up"),
          ),
          control(
            "down",
            index === rows.length - 1 ? "setup_account_last" : null,
            () => {
              move(editor, hands, row.id, "down");
            },
            holds.control(row.id, "down"),
          ),
          control("edit", null, () => {
            edit(editor, row);
          }),
          control("remove", rows.length <= 1 ? "setup_account_only" : null, () => {
            remove(editor, hands, row.id);
          }),
        ],
      };
    }),
    retries:
      counted === undefined || rows.length < 2
        ? undefined
        : {
            label: say(lang, "setup_account_retries"),
            options: (["one", "two"] as const).map((value) => ({
              value,
              label: say(lang, RETRY_WORDS[value]),
              ...(busy === undefined ? {} : { why: busy }),
            })),
            held: shownRetries(editor, roster),
            fallback:
              counted.fallback === undefined || shownRetries(editor, roster) !== null
                ? undefined
                : fill(say(lang, "setup_account_retries_fallback"), {
                    n: say(lang, RETRY_WORDS[counted.fallback]),
                  }),
            pick: (retries) => {
              pickRetries(editor, hands, retries);
            },
          },
    title:
      editor.editing === null
        ? say(lang, "setup_account_add")
        : fill(say(lang, "setup_account_editing"), { id: editor.editing }),
    fields: [
      field("id", "setup_account_id", {
        disabled: busy !== undefined || editor.editing !== null,
      }),
      field("key", "setup_key", {
        secret: true,
        disabled: busy !== undefined || environmental,
        help: keyHelp === null ? undefined : say(lang, keyHelp),
      }),
      field("reference", "setup_account_reference", { folded: true }),
      field("header", "setup_account_header", { folded: roster.kind === "provider" }),
    ],
    more: say(lang, "setup_account_more"),
    route: say(lang, "setup_account_key_route"),
    save: {
      label: say(lang, "setup_account_save"),
      why:
        busy ??
        (unsaved === null || unsaved === "setup_account_busy" ? undefined : say(lang, unsaved)),
      loading: editor.pending !== null || hands.flight?.current?.kind === "accounts",
      press: () => {
        save(editor, hands);
      },
    },
    cancel:
      editor.editing === null
        ? undefined
        : {
            label: say(lang, "setup_account_cancel"),
            press: () => {
              cancel(editor);
            },
          },
    forget:
      editor.removed === null
        ? undefined
        : {
            text: fill(say(lang, "setup_account_forget_offer"), { id: editor.removed.id }),
            label: say(lang, "setup_account_forget"),
            press: () => {
              forget(editor, hands);
            },
          },
    note: editor.note ?? undefined,
    refused: editor.refused ?? undefined,
  };
}
