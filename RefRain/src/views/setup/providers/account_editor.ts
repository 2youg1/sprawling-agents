// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import { forgetSecret } from "../../../core/enrol";
import type { Enrolling, Enrolment, SecretAt } from "../../../core/enrol";
import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import type {
  AccountRetries,
  AccountStatus,
  AxError,
  Command,
  ProviderAccount,
} from "../../../wire";
import { begin, end } from "./flight";
import type { Flight, FlightState } from "./flight";

export type Step = "up" | "down";

export type Owner = "provider" | "supplier";

export interface Roster {
  readonly kind: Owner;
  readonly owner: string;
  readonly accounts: readonly ProviderAccount[] | null;
  readonly keys: readonly AccountStatus[];
  readonly legacy: boolean;
  readonly filed: (account: string) => SecretAt;
  readonly frame: (rows: readonly ProviderAccount[], retries: AccountRetries | null) => Command;
  readonly retries: RetryCount | undefined;
  readonly stamp: string;
}

export interface RetryCount {
  readonly chosen: AccountRetries | null;
  readonly fallback: AccountRetries | undefined;
}

export interface AccountDraft {
  readonly id: string;
  readonly reference: string;
  readonly header: string;
  readonly key: string;
}

export interface Filing {
  readonly ticket: symbol;
  readonly account: ProviderAccount;
  readonly owner: string;
  readonly origin: string;
  readonly token: string | null;
}

export interface Sent {
  readonly rows: readonly ProviderAccount[];
  readonly retries: AccountRetries | null;
  readonly against: string;
  readonly flight: { readonly state: FlightState; readonly operation: Flight } | null;
}

export interface Removed {
  readonly id: string;
  readonly reference: string;
}

export interface Editor {
  draft: AccountDraft;
  editing: string | null;
  note: string | null;
  refused: AxError | null;
  pending: Filing | null;
  sent: Sent | null;
  removed: Removed | null;
  forgetting: boolean;
}

export interface Reach {
  readonly origin: string;
  readonly token: string | null;
}

export interface Hands {
  readonly roster: () => Roster;
  readonly lang: () => Lang;
  readonly reach: () => Reach;
  readonly send: (command: Command) => boolean;
  readonly enrol: (at: Enrolling) => Promise<Enrolment>;
  readonly follow: (id: string, step: Step) => void;
  readonly focusHeading: () => void;
  readonly flight?: FlightState;
}

const BLANK: AccountDraft = { id: "", reference: "", header: "", key: "" };

export function freshEditor(): Editor {
  return {
    draft: BLANK,
    editing: null,
    note: null,
    refused: null,
    pending: null,
    sent: null,
    removed: null,
    forgetting: false,
  };
}

const labelled = Schema.is(ServerLabel);

export function shown(editor: Editor, roster: Roster): readonly ProviderAccount[] {
  return editor.sent?.rows ?? roster.accounts ?? [];
}

export function shownRetries(editor: Editor, roster: Roster): AccountRetries | null {
  return editor.sent === null ? (roster.retries?.chosen ?? null) : editor.sent.retries;
}

export function settle(editor: Editor, roster: Roster): void {
  if (editor.sent === null || editor.sent.against === roster.stamp) return;
  release(editor.sent);
  editor.sent = null;
}

function release(sent: Sent): void {
  if (sent.flight !== null) end(sent.flight.state, sent.flight.operation);
}

export function refuse(editor: Editor, error: AxError): void {
  if (editor.sent !== null) release(editor.sent);
  editor.sent = null;
  editor.forgetting = false;
  editor.refused = error;
}

export function awaits(editor: Editor): boolean {
  return editor.sent !== null || editor.forgetting;
}

export function leave(editor: Editor): void {
  if (editor.sent !== null) release(editor.sent);
  editor.sent = null;
  editor.pending = null;
  editor.forgetting = false;
}

export function input(editor: Editor, part: keyof AccountDraft, value: string): void {
  editor.draft = { ...editor.draft, [part]: value };
}

export function edit(editor: Editor, account: ProviderAccount): void {
  if (editor.pending !== null) return;
  editor.editing = account.id;
  editor.note = null;
  editor.draft = {
    id: account.id,
    reference: account.reference ?? "",
    header: account.header ?? "",
    key: "",
  };
}

export function cancel(editor: Editor): void {
  if (editor.pending !== null) return;
  editor.editing = null;
  editor.note = null;
  editor.draft = BLANK;
}

export function unsaveable(editor: Editor, rows: readonly ProviderAccount[]): Key | null {
  const id = editor.draft.id.trim();
  if (editor.pending !== null) return "setup_account_busy";
  if (!labelled(id)) return "setup_account_id_shape";
  if (editor.editing === null && rows.some((row) => row.id === id)) return "setup_account_id_taken";
  return null;
}

export function save(editor: Editor, hands: Hands): void {
  if (hands.flight?.current !== null && hands.flight?.current !== undefined) return;
  const roster = hands.roster();
  if (unsaveable(editor, shown(editor, roster)) !== null) return;
  const { draft } = editor;
  const account: ProviderAccount = {
    id: ServerLabel.make(draft.id.trim()),
    reference: draft.reference.trim() === "" ? null : draft.reference.trim(),
    header: draft.header.trim() === "" ? null : draft.header.trim(),
  };
  editor.note = null;
  if (draft.key === "") {
    store(editor, hands, account);
    return;
  }
  const filing: Filing = {
    ticket: Symbol("filing"),
    account,
    owner: roster.owner,
    ...hands.reach(),
  };
  const at = roster.filed(account.id);
  editor.pending = filing;
  void hands
    .enrol({
      origin: filing.origin,
      token: filing.token,
      realm: at.realm,
      name: at.name,
      value: draft.key,
      lang: hands.lang(),
    })
    .then((outcome) => {
      if (editor.pending?.ticket !== filing.ticket) return;
      editor.pending = null;
      const now = hands.reach();
      if (
        hands.roster().owner !== filing.owner ||
        now.origin !== filing.origin ||
        now.token !== filing.token
      )
        return;
      if (outcome.kind === "stored")
        store(editor, hands, { ...filing.account, reference: outcome.reference });
      else editor.note = outcome.reason;
    });
}

function store(editor: Editor, hands: Hands, account: ProviderAccount): void {
  const rows = shown(editor, hands.roster());
  const at = rows.findIndex((row) => row.id === account.id);
  const next =
    at < 0 ? [...rows, account] : rows.map((row, index) => (index === at ? account : row));
  if (!commit(editor, hands, next)) return;
  editor.editing = null;
  editor.draft = BLANK;
}

export function move(editor: Editor, hands: Hands, id: string, step: Step): void {
  if (editor.pending !== null) return;
  const rows = shown(editor, hands.roster());
  const from = rows.findIndex((row) => row.id === id);
  const to = step === "up" ? from - 1 : from + 1;
  const held = rows[from];
  const other = rows[to];
  if (from < 0 || held === undefined || other === undefined) return;
  const next = rows.map((row, index) => (index === from ? other : index === to ? held : row));
  if (commit(editor, hands, next)) hands.follow(id, step);
}

export function remove(editor: Editor, hands: Hands, id: string): void {
  if (editor.pending !== null) return;
  const roster = hands.roster();
  const rows = shown(editor, roster);
  if (rows.length <= 1) return;
  const reference = rows.find((row) => row.id === id)?.reference ?? null;
  const stored = roster.keys.some((status) => status.id === id && status.key === "stored");
  if (
    !commit(
      editor,
      hands,
      rows.filter((row) => row.id !== id),
    )
  )
    return;
  editor.removed = reference !== null && stored ? { id, reference } : null;
  if (editor.editing === id) {
    editor.editing = null;
    editor.draft = BLANK;
  }
  hands.focusHeading();
}

export function forget(editor: Editor, hands: Hands): void {
  const removed = editor.removed;
  if (removed === null) return;
  if (!hands.send(forgetSecret(removed.reference))) {
    editor.note = say(hands.lang(), "setup_account_not_sent");
    return;
  }
  editor.removed = null;
  editor.forgetting = true;
  editor.refused = null;
  editor.note = say(hands.lang(), "setup_account_forget_sent");
}

export function pickRetries(editor: Editor, hands: Hands, retries: AccountRetries): void {
  if (editor.pending !== null) return;
  const roster = hands.roster();
  if (roster.retries === undefined) return;
  commit(editor, hands, shown(editor, roster), retries);
}

function commit(
  editor: Editor,
  hands: Hands,
  rows: readonly ProviderAccount[],
  retries: AccountRetries | null = shownRetries(editor, hands.roster()),
): boolean {
  const roster = hands.roster();
  const operation = hands.flight === undefined ? null : begin(hands.flight, "accounts");
  if (hands.flight !== undefined && operation === null) return false;
  if (!hands.send(roster.frame(rows, retries))) {
    if (operation !== null && hands.flight !== undefined) end(hands.flight, operation);
    editor.note = say(hands.lang(), "setup_account_not_sent");
    return false;
  }
  editor.note = null;
  editor.refused = null;
  editor.removed = null;
  editor.forgetting = false;
  editor.sent = {
    rows,
    retries,
    against: roster.stamp,
    flight:
      hands.flight === undefined || operation === null ? null : { state: hands.flight, operation },
  };
  return true;
}
