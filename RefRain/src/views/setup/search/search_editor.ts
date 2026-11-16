// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import { configureSearch } from "../../../core/commands";
import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import type {
  AxError,
  Command,
  SearchConfiguration,
  SearchSupplier,
  SettledSearch,
} from "../../../wire";
import type { CustomSearch } from "../providers/rosters";

export type Choice = "default" | "custom" | "off";

export interface SupplierDraft {
  readonly id: string;
  readonly url: string;
  readonly remote: string;
  readonly query: string;
  readonly count: string;
  readonly objective: string;
}

export interface SearchEditor {
  draft: SupplierDraft;
  editing: string | null;
  adding: boolean;
  kept: CustomSearch | null;
  note: string | null;
  refused: AxError | null;
  sent: { readonly value: SearchConfiguration; readonly against: string } | null;
}

export interface SearchHands {
  readonly settled: () => SettledSearch;
  readonly lang: () => Lang;
  readonly send: (command: Command) => boolean;
}

const BLANK: SupplierDraft = { id: "", url: "", remote: "", query: "", count: "", objective: "" };

export function freshSearch(): SearchEditor {
  return {
    draft: BLANK,
    editing: null,
    adding: false,
    kept: null,
    note: null,
    refused: null,
    sent: null,
  };
}

const labelled = Schema.is(ServerLabel);

export function stampOf(settled: SettledSearch): string {
  return JSON.stringify(settled);
}

export function shownSearch(editor: SearchEditor, settled: SettledSearch): SearchConfiguration {
  return editor.sent?.value ?? settled.city ?? "default";
}

export function choiceOf(value: SearchConfiguration): Choice {
  return typeof value === "string" ? value : "custom";
}

export function customOf(value: SearchConfiguration): CustomSearch | null {
  return typeof value === "string" ? null : value.custom;
}

export function settle(editor: SearchEditor, settled: SettledSearch): void {
  if (editor.sent !== null && editor.sent.against !== stampOf(settled)) editor.sent = null;
}

export function refuse(editor: SearchEditor, error: AxError): void {
  editor.sent = null;
  editor.refused = error;
}

export function choose(editor: SearchEditor, hands: SearchHands, choice: Choice): void {
  const now = shownSearch(editor, hands.settled());
  if (choiceOf(now) === choice) return;
  editor.kept = customOf(now) ?? editor.kept;
  if (choice !== "custom") {
    editor.adding = false;
    commit(editor, hands, choice);
    return;
  }
  if (editor.kept === null) {
    editor.adding = true;
    return;
  }
  commit(editor, hands, { custom: editor.kept });
}

export function input(editor: SearchEditor, part: keyof SupplierDraft, value: string): void {
  editor.draft = { ...editor.draft, [part]: value };
}

export function edit(editor: SearchEditor, supplier: SearchSupplier): void {
  editor.editing = supplier.id;
  editor.note = null;
  editor.draft = {
    id: supplier.id,
    url: supplier.url,
    remote: supplier.remote,
    query: supplier.query_field,
    count: supplier.count_field ?? "",
    objective: supplier.objective_field ?? "",
  };
}

export function cancel(editor: SearchEditor): void {
  editor.editing = null;
  editor.adding = false;
  editor.note = null;
  editor.draft = BLANK;
}

export function unsaveable(editor: SearchEditor, custom: CustomSearch | null): Key | null {
  const { draft } = editor;
  if (!labelled(draft.id.trim())) return "setup_account_id_shape";
  if (
    editor.editing === null &&
    custom?.suppliers.some((each) => each.id === draft.id.trim()) === true
  )
    return "search_id_taken";
  if ([draft.url, draft.remote, draft.query].some((part) => part.trim() === ""))
    return "search_required";
  return null;
}

export function save(editor: SearchEditor, hands: SearchHands): void {
  const custom = customOf(shownSearch(editor, hands.settled()));
  if (unsaveable(editor, custom) !== null) return;
  const { draft } = editor;
  const optional = (text: string): string | null => (text.trim() === "" ? null : text.trim());
  const listed = custom?.suppliers ?? [];
  const id = ServerLabel.make(draft.id.trim());
  const supplier: SearchSupplier = {
    id,
    url: draft.url.trim(),
    remote: draft.remote.trim(),
    query_field: draft.query.trim(),
    count_field: optional(draft.count),
    objective_field: optional(draft.objective),
    accounts: listed.find((each) => each.id === id)?.accounts ?? [],
  };
  const at = listed.findIndex((each) => each.id === id);
  const suppliers =
    at < 0 ? [...listed, supplier] : listed.map((each, index) => (index === at ? supplier : each));
  if (!commit(editor, hands, { custom: { selected: custom?.selected ?? id, suppliers } })) return;
  cancel(editor);
}

export function select(editor: SearchEditor, hands: SearchHands, id: ServerLabel): void {
  const custom = customOf(shownSearch(editor, hands.settled()));
  if (custom === null || custom.selected === id) return;
  commit(editor, hands, { custom: { ...custom, selected: id } });
}

export function remove(editor: SearchEditor, hands: SearchHands, id: ServerLabel): void {
  const custom = customOf(shownSearch(editor, hands.settled()));
  if (custom === null || custom.selected === id || custom.suppliers.length <= 1) return;
  if (
    !commit(editor, hands, {
      custom: { ...custom, suppliers: custom.suppliers.filter((each) => each.id !== id) },
    })
  )
    return;
  if (editor.editing === id) cancel(editor);
}

function commit(editor: SearchEditor, hands: SearchHands, value: SearchConfiguration): boolean {
  const settled = hands.settled();
  if (!hands.send(configureSearch(value))) {
    editor.note = say(hands.lang(), "setup_account_not_sent");
    return false;
  }
  editor.note = null;
  editor.refused = null;
  editor.kept = customOf(value) ?? editor.kept;
  editor.sent = { value, against: stampOf(settled) };
  return true;
}
