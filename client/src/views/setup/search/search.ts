// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the web search card gives its look: every word translated, every
// press wired to `./search_editor`, every control that cannot be used
// now with the reason why, and for each service the roster its account
// editor is seated with. This value is the whole contract between the
// wiring and a look (client D93).

import { fill, say } from "../../../core/lang";
import type { Key } from "../../../core/lang";
import type { AxError, ConfigLayer } from "../../../wire";
import type { Roster } from "../providers/account_editor";
import { supplierRoster } from "../providers/rosters";
import { cancel, choiceOf, choose, customOf, edit, input, remove, save, select, shownSearch, stampOf, unsaveable } from "./search_editor";
import type { Choice, SearchEditor, SearchHands, SupplierDraft } from "./search_editor";

export interface ChoiceLook {
  readonly label: string;
  readonly options: readonly { readonly value: Choice; readonly label: string }[];
  readonly held: Choice;
  readonly pick: (choice: Choice) => void;
}

export type SupplierControl = "use" | "edit" | "remove";

export interface SupplierRowLook {
  readonly id: string;
  readonly url: string;
  readonly remote: string;
  readonly selected: boolean;
  readonly controls: readonly {
    readonly key: SupplierControl;
    readonly label: string;
    // Present means the control cannot be used now, and says why.
    readonly why: string | undefined;
    readonly press: () => void;
  }[];
  readonly accounts: string;
  readonly roster: Roster;
}

export interface SupplierFieldLook {
  readonly name: keyof SupplierDraft;
  readonly label: string;
  readonly value: string;
  readonly disabled: boolean;
  readonly input: (value: string) => void;
}

export interface SupplierFormLook {
  readonly title: string;
  // The form stands open while a service is being edited or the first
  // one added; otherwise it folds under its title, because the list is
  // read far more often than a service is added.
  readonly open: boolean;
  readonly fields: readonly SupplierFieldLook[];
  readonly save: { readonly label: string; readonly why: string | undefined; readonly press: () => void };
  readonly cancel: { readonly label: string; readonly press: () => void } | undefined;
}

export interface SearchLook {
  readonly heading: string;
  readonly intro: string;
  readonly choice: ChoiceLook;
  readonly defaultAt: string;
  // A file nearer the hall states its own `[search]`, which this card
  // does not change.
  readonly override: string | undefined;
  readonly confidential: string;
  // Present while the city's layer is custom.
  readonly noFallback: string | undefined;
  readonly suppliersHeading: string;
  readonly suppliers: readonly SupplierRowLook[];
  readonly form: SupplierFormLook | undefined;
  readonly note: string | undefined;
  readonly refused: AxError | undefined;
}

const CHOICES: readonly Choice[] = ["default", "custom", "off"];

const CHOICE_WORDS: Record<Choice, Key> = {
  default: "search_choice_default",
  custom: "search_choice_custom",
  off: "search_choice_off",
};

const LAYERS: Record<ConfigLayer, Key> = {
  default: "search_layer_default",
  city: "search_layer_city",
  building: "search_layer_building",
  resident: "search_layer_resident",
};

const FIELDS: readonly (readonly [keyof SupplierDraft, Key])[] = [
  ["id", "search_id"],
  ["url", "search_url"],
  ["remote", "search_remote"],
  ["query", "search_query"],
  ["count", "search_count"],
  ["objective", "search_objective"],
];

export function lookOf(editor: SearchEditor, hands: SearchHands): SearchLook {
  const settled = hands.settled();
  const lang = hands.lang();
  const shown = shownSearch(editor, settled);
  const custom = customOf(shown);
  const stamp = stampOf(settled);
  const unsaved = unsaveable(editor, custom);
  const control = (key: SupplierControl, label: Key, why: Key | null, press: () => void) => ({
    key,
    label: say(lang, label),
    why: why === null ? undefined : say(lang, why),
    press,
  });
  return {
    heading: say(lang, "search_card"),
    intro: say(lang, "search_card_note"),
    choice: {
      label: say(lang, "search_card"),
      options: CHOICES.map((value) => ({ value, label: say(lang, CHOICE_WORDS[value]) })),
      held: editor.adding ? "custom" : choiceOf(shown),
      pick: (choice) => {
        choose(editor, hands, choice);
      },
    },
    defaultAt: fill(say(lang, "search_default_at"), { url: settled.default_url }),
    override:
      settled.from === "building" || settled.from === "resident"
        ? fill(say(lang, "search_override"), { choice: say(lang, CHOICE_WORDS[choiceOf(settled.configuration)]), layer: say(lang, LAYERS[settled.from]) })
        : undefined,
    confidential: say(lang, "search_confidential"),
    noFallback: custom === null ? undefined : say(lang, "search_no_fallback"),
    suppliersHeading: say(lang, "search_suppliers"),
    suppliers: (custom === null ? [] : custom.suppliers).map((supplier) => {
      const selected = supplier.id === custom?.selected;
      const only = (custom?.suppliers.length ?? 0) <= 1;
      return {
        id: supplier.id,
        url: supplier.url,
        remote: supplier.remote,
        selected,
        controls: [
          control("use", "search_use", selected ? "search_in_use" : null, () => { select(editor, hands, supplier.id); }),
          control("edit", "search_edit", null, () => { edit(editor, supplier); }),
          control("remove", "search_remove", only ? "search_only" : selected ? "search_remove_in_use" : null, () => { remove(editor, hands, supplier.id); }),
        ],
        accounts: fill(say(lang, "setup_accounts_toggle"), { n: String(supplier.accounts.length) }),
        roster: supplierRoster(
          custom ?? { selected: supplier.id, suppliers: [supplier] },
          supplier,
          settled.account_status.find((each) => each.supplier === supplier.id)?.accounts ?? [],
          stamp,
        ),
      };
    }),
    form:
      custom === null && !editor.adding
        ? undefined
        : {
            title: editor.editing === null ? say(lang, "search_add") : fill(say(lang, "search_editing"), { id: editor.editing }),
            open: editor.editing !== null || editor.adding,
            fields: FIELDS.map(([name, label]) => ({
              name,
              label: say(lang, label),
              value: editor.draft[name],
              disabled: name === "id" && editor.editing !== null,
              input: (value: string) => {
                input(editor, name, value);
              },
            })),
            save: { label: say(lang, "search_save"), why: unsaved === null ? undefined : say(lang, unsaved), press: () => { save(editor, hands); } },
            cancel: editor.editing === null && !editor.adding ? undefined : { label: say(lang, "setup_account_cancel"), press: () => { cancel(editor); } },
          },
    note: editor.note ?? undefined,
    refused: editor.refused ?? undefined,
  };
}
