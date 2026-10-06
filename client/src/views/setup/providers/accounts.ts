// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one endpoint's account editor gives its look: every word
// translated, every press wired to `./account_editor`, every control
// that cannot be used now with the reason why. This value is the whole
// contract between the wiring and a look, so another look - one built
// from a component library - draws the same editor by taking it
// (client D92).

import type { Attachment } from "svelte/attachments";

import { fill, say } from "../../../core/lang";
import type { Key } from "../../../core/lang";
import type { AxError, EndpointSummary, KeyState } from "../../../wire";
import { cancel, edit, input, move, remove, save, shown, unsaveable } from "./account_editor";
import type { Editor, Hands, Step } from "./account_editor";

export type Control = "up" | "down" | "edit" | "remove";

export interface ControlLook {
  readonly key: Control;
  readonly label: string;
  // Present means the control cannot be used now, and says why.
  readonly why: string | undefined;
  readonly press: () => void;
  // Put on the element that holds the control, so focus can be put back
  // on it after the row moved; absent on controls focus never returns to.
  readonly hold: Attachment<HTMLElement> | undefined;
}

export interface RowLook {
  readonly id: string;
  readonly place: number;
  readonly key: string;
  // The key is not there or cannot be used, which is what the person
  // has to act on.
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
  // Folded under `more` rather than standing in the form.
  readonly folded: boolean;
  readonly input: (value: string) => void;
}

export interface AccountsLook {
  readonly heading: string;
  readonly holdHeading: Attachment<HTMLElement>;
  readonly order: string;
  readonly legacy: string | undefined;
  readonly rows: readonly RowLook[];
  readonly title: string;
  readonly fields: readonly FieldLook[];
  readonly more: string;
  readonly route: string;
  readonly save: { readonly label: string; readonly why: string | undefined; readonly loading: boolean; readonly press: () => void };
  readonly cancel: { readonly label: string; readonly press: () => void } | undefined;
  readonly note: string | undefined;
  readonly refused: AxError | undefined;
}

// Where the look puts focus back to.
export interface Holds {
  readonly heading: Attachment<HTMLElement>;
  readonly control: (id: string, step: Step) => Attachment<HTMLElement>;
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

// What the city said about one account's key. An account the answer
// does not list yet - one just sent - has not been looked at.
function keyOf(endpoint: EndpointSummary, id: string): KeyState {
  return endpoint.account_status.find((status) => status.id === id)?.key ?? "unread";
}

export function lookOf(editor: Editor, hands: Hands, holds: Holds): AccountsLook {
  const endpoint = hands.endpoint();
  const lang = hands.lang();
  const rows = shown(editor, endpoint);
  const busy = editor.pending === null ? undefined : say(lang, "setup_account_busy");
  const control = (key: Control, why: Key | null, press: () => void, hold?: Attachment<HTMLElement>): ControlLook => ({
    key,
    label: say(lang, `setup_account_${key}`),
    why: busy ?? (why === null ? undefined : say(lang, why)),
    press,
    hold,
  });
  const environmental = editor.editing !== null && keyOf(endpoint, editor.editing).startsWith("environment");
  const unsaved = unsaveable(editor, rows);
  const field = (name: FieldName, label: Key, more: Partial<FieldLook> = {}): FieldLook => ({
    name,
    label: say(lang, label),
    value: editor.draft[name],
    secret: false,
    mono: true,
    disabled: editor.pending !== null,
    help: undefined,
    folded: false,
    input: (value) => {
      input(editor, name, value);
    },
    ...more,
  });
  const keyHelp = environmental ? "setup_account_key_environment_help" : editor.editing === null ? null : "setup_account_key_kept";
  return {
    heading: say(lang, "setup_accounts"),
    holdHeading: holds.heading,
    order: say(lang, "setup_accounts_order"),
    legacy: (endpoint.tuning.accounts ?? null) === null && endpoint.has_credential ? say(lang, "setup_accounts_legacy") : undefined,
    rows: rows.map((row, index) => {
      const key = keyOf(endpoint, row.id);
      return {
        id: row.id,
        place: index + 1,
        key: say(lang, KEY_WORDS[key]),
        wanting: WANTING[key],
        controls: [
          control("up", index === 0 ? "setup_account_first" : null, () => { move(editor, hands, row.id, "up"); }, holds.control(row.id, "up")),
          control("down", index === rows.length - 1 ? "setup_account_last" : null, () => { move(editor, hands, row.id, "down"); }, holds.control(row.id, "down")),
          control("edit", null, () => { edit(editor, row); }),
          control("remove", rows.length <= 1 ? "setup_account_only" : null, () => { remove(editor, hands, row.id); }),
        ],
      };
    }),
    title: editor.editing === null ? say(lang, "setup_account_add") : fill(say(lang, "setup_account_editing"), { id: editor.editing }),
    fields: [
      field("id", "setup_account_id", { disabled: editor.pending !== null || editor.editing !== null }),
      field("key", "setup_key", {
        secret: true,
        disabled: editor.pending !== null || environmental,
        help: keyHelp === null ? undefined : say(lang, keyHelp),
      }),
      field("reference", "setup_account_reference", { folded: true }),
      field("header", "setup_account_header", { folded: true }),
    ],
    more: say(lang, "setup_account_more"),
    route: say(lang, "setup_account_key_route"),
    save: {
      label: say(lang, "setup_account_save"),
      why: unsaved === null || unsaved === "setup_account_busy" ? undefined : say(lang, unsaved),
      loading: editor.pending !== null,
      press: () => {
        save(editor, hands);
      },
    },
    cancel: editor.editing === null ? undefined : { label: say(lang, "setup_account_cancel"), press: () => { cancel(editor); } },
    note: editor.note ?? undefined,
    refused: editor.refused ?? undefined,
  };
}
