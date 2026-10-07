// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The state of one account editor and what each press does to it: the
// draft, the key on its way to the vault, and the list sent to the
// city. The editor does not know whose list it edits - a provider's or
// a search supplier's - only the `Roster` it is lent (`./rosters`). Nothing here touches the DOM or uses a rune, so the seat
// (`accounts.svelte`) holds `Editor` in `$state` and the tests hold it
// in a plain object; `accounts.ts` turns it into the value a look draws
// (client D93).
//
// **The order of the list is the whole priority** (gateway Router D28):
// a move swaps two rows, and every change sends the complete list in
// the frame the roster names. What was sent is drawn until the city's next
// answer for this owner arrives, so a second move starts from the
// first one's result rather than from the order the city had before.

import { Schema } from "effect";

import { forgetSecret } from "../../../core/enrol";
import type { Enrolling, Enrolment, SecretAt } from "../../../core/enrol";
import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import type { AccountRetries, AccountStatus, AxError, Command, ProviderAccount } from "../../../wire";

export type Step = "up" | "down";

// Who owns the list, which changes what the editor says about the
// order and whether the header name stands in the form.
export type Owner = "provider" | "supplier";

// One owner of an ordered account list, as the city last answered it.
export interface Roster {
  readonly kind: Owner;
  // The provider or supplier id; a key that comes back from the vault
  // after this changed lands nowhere.
  readonly owner: string;
  // The list as answered; null while an endpoint still calls with the
  // one key it was attached with.
  readonly accounts: readonly ProviderAccount[] | null;
  readonly keys: readonly AccountStatus[];
  // The endpoint was attached with a key and lists no account yet.
  readonly legacy: boolean;
  // Where a key typed for `account` is filed in the vault.
  readonly filed: (account: string) => SecretAt;
  // The one frame that carries the whole list, with the per-account
  // retry count an owner that has one sends beside it.
  readonly frame: (rows: readonly ProviderAccount[], retries: AccountRetries | null) => Command;
  // The per-account retry count, for an owner that has one.
  readonly retries: RetryCount | undefined;
  // The answer this roster was read from, written out so a newer one is
  // told from it by content.
  readonly stamp: string;
}

export interface RetryCount {
  readonly chosen: AccountRetries | null;
  // What the city calls with when none is chosen; undefined until the
  // city answered.
  readonly fallback: AccountRetries | undefined;
}

// What the add-or-edit form holds, as typed.
export interface AccountDraft {
  readonly id: string;
  readonly reference: string;
  readonly header: string;
  readonly key: string;
}

// A key on its way to the vault, with everything that has to be the
// same when the answer comes back for the answer to still be this
// form's: the account it was typed for, the provider, and the city and
// pairing the page was talking to.
//
// **One filing is told from another by its ticket, never by identity.**
// The seat holds the editor in `$state`, which hands back a proxy of
// whatever object was put in it, so an object compared with itself
// after the round trip is a different object.
export interface Filing {
  readonly ticket: symbol;
  readonly account: ProviderAccount;
  readonly owner: string;
  readonly origin: string;
  readonly token: string | null;
}

// A list this editor sent, with the retry count sent beside it, and the
// answer it was sent against (see `Filing` for why that is compared by
// content).
export interface Sent {
  readonly rows: readonly ProviderAccount[];
  readonly retries: AccountRetries | null;
  readonly against: string;
}

// An account just removed whose key the vault still holds: the editor
// offers to delete the key, and the offer stands until the next press.
export interface Removed {
  readonly id: string;
  readonly reference: string;
}

export interface Editor {
  draft: AccountDraft;
  // The id of the account the form is editing; null while adding.
  editing: string | null;
  note: string | null;
  refused: AxError | null;
  pending: Filing | null;
  sent: Sent | null;
  removed: Removed | null;
  // A deletion was sent, so a refusal that arrives is this editor's.
  forgetting: boolean;
}

// Where the city and pairing the page talks to stand right now.
export interface Reach {
  readonly origin: string;
  readonly token: string | null;
}

// Everything the wiring needs from outside: the roster as now
// answered, the language, the door to the city and to the vault, and
// the two places focus is put back.
export interface Hands {
  readonly roster: () => Roster;
  readonly lang: () => Lang;
  readonly reach: () => Reach;
  readonly send: (command: Command) => boolean;
  readonly enrol: (at: Enrolling) => Promise<Enrolment>;
  // After a move: the same control on the row that moved.
  readonly follow: (id: string, step: Step) => void;
  // After a removal: the editor's heading.
  readonly focusHeading: () => void;
}

const BLANK: AccountDraft = { id: "", reference: "", header: "", key: "" };

export function freshEditor(): Editor {
  return { draft: BLANK, editing: null, note: null, refused: null, pending: null, sent: null, removed: null, forgetting: false };
}

const labelled = Schema.is(ServerLabel);

// The list the editor draws: the one it sent while that is newer than
// the city's answer, and the city's list otherwise. An endpoint
// attached with one legacy key lists none.
export function shown(editor: Editor, roster: Roster): readonly ProviderAccount[] {
  return editor.sent?.rows ?? roster.accounts ?? [];
}

// The retry count the editor draws, by the same rule as the list.
export function shownRetries(editor: Editor, roster: Roster): AccountRetries | null {
  return editor.sent === null ? (roster.retries?.chosen ?? null) : editor.sent.retries;
}

// A newer answer for this owner replaces what was sent.
export function settle(editor: Editor, roster: Roster): void {
  if (editor.sent !== null && editor.sent.against !== roster.stamp) editor.sent = null;
}

// The city refused the list: the editor draws the city's list again and
// says why beside it.
export function refuse(editor: Editor, error: AxError): void {
  editor.sent = null;
  editor.forgetting = false;
  editor.refused = error;
}

// Whether a refusal that arrived now answers something this editor sent.
export function awaits(editor: Editor): boolean {
  return editor.sent !== null || editor.forgetting;
}

// Leaving the editor: a key still on its way lands nowhere, and a
// refusal that arrives later is no longer this editor's to show, since
// the city answers a deletion that succeeds with nothing.
export function leave(editor: Editor): void {
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
  editor.draft = { id: account.id, reference: account.reference ?? "", header: account.header ?? "", key: "" };
}

export function cancel(editor: Editor): void {
  if (editor.pending !== null) return;
  editor.editing = null;
  editor.note = null;
  editor.draft = BLANK;
}

// Why the form cannot be saved as it stands, or null when it can.
export function unsaveable(editor: Editor, rows: readonly ProviderAccount[]): Key | null {
  const id = editor.draft.id.trim();
  if (editor.pending !== null) return "setup_account_busy";
  if (!labelled(id)) return "setup_account_id_shape";
  if (editor.editing === null && rows.some((row) => row.id === id)) return "setup_account_id_taken";
  return null;
}

// Saves the form. A typed key is filed in the vault first, under the
// one name the roster gives this owner and account, and the account
// carries the reference the city answers with.
export function save(editor: Editor, hands: Hands): void {
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
  const filing: Filing = { ticket: Symbol("filing"), account, owner: roster.owner, ...hands.reach() };
  const at = roster.filed(account.id);
  editor.pending = filing;
  void hands
    .enrol({ origin: filing.origin, token: filing.token, realm: at.realm, name: at.name, value: draft.key, lang: hands.lang() })
    .then((outcome) => {
      if (editor.pending?.ticket !== filing.ticket) return;
      editor.pending = null;
      const now = hands.reach();
      if (hands.roster().owner !== filing.owner || now.origin !== filing.origin || now.token !== filing.token) return;
      if (outcome.kind === "stored") store(editor, hands, { ...filing.account, reference: outcome.reference });
      else editor.note = outcome.reason;
    });
}

// Puts one account into the list - in its own place when it is listed,
// last when it is new - and sends the list. The form empties only when
// the list went out.
function store(editor: Editor, hands: Hands, account: ProviderAccount): void {
  const rows = shown(editor, hands.roster());
  const at = rows.findIndex((row) => row.id === account.id);
  const next = at < 0 ? [...rows, account] : rows.map((row, index) => (index === at ? account : row));
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
  if (!commit(editor, hands, rows.filter((row) => row.id !== id))) return;
  // Only a key the vault holds is offered: one the environment supplies
  // is refused by the city (gateway D33), and an anonymous account has
  // none.
  editor.removed = reference !== null && stored ? { id, reference } : null;
  if (editor.editing === id) {
    editor.editing = null;
    editor.draft = BLANK;
  }
  hands.focusHeading();
}

// Deletes the key of the account just removed. Sent after the list that
// no longer names it, on the same link, so the city reads the two in
// that order; were the list refused, this is refused too, because the
// reference is still named.
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

// Picks the per-account retry count: the same frame, the list as drawn.
export function pickRetries(editor: Editor, hands: Hands, retries: AccountRetries): void {
  if (editor.pending !== null) return;
  const roster = hands.roster();
  if (roster.retries === undefined) return;
  commit(editor, hands, shown(editor, roster), retries);
}

// Sends the whole list. A frame that could not leave keeps the draft
// and says so; one that left is drawn until the city answers.
function commit(
  editor: Editor,
  hands: Hands,
  rows: readonly ProviderAccount[],
  retries: AccountRetries | null = shownRetries(editor, hands.roster()),
): boolean {
  const roster = hands.roster();
  if (!hands.send(roster.frame(rows, retries))) {
    editor.note = say(hands.lang(), "setup_account_not_sent");
    return false;
  }
  editor.note = null;
  editor.refused = null;
  editor.removed = null;
  editor.forgetting = false;
  editor.sent = { rows, retries, against: roster.stamp };
  return true;
}
