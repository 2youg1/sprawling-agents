// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The state of one endpoint's account editor and what each press does
// to it: the draft, the key on its way to the vault, and the list sent
// to the city. Nothing here touches the DOM or uses a rune, so the seat
// (`accounts.svelte`) holds `Editor` in `$state` and the tests hold it
// in a plain object; `accounts.ts` turns it into the value a look draws
// (client D92).
//
// **The order of the list is the whole priority** (gateway Router D28):
// a move swaps two rows, and every change sends the complete list
// through `AttachEndpoint`. What was sent is drawn until the city's next
// answer for this endpoint arrives, so a second move starts from the
// first one's result rather than from the order the city had before.

import { Schema } from "effect";

import { reattachEndpoint } from "../../../core/commands";
import { referenceFor } from "../../../core/enrol";
import type { Enrolling, Enrolment } from "../../../core/enrol";
import { say } from "../../../core/lang";
import type { Key, Lang } from "../../../core/lang";
import { ServerLabel } from "../../../wire";
import type { AxError, Command, EndpointSummary, ProviderAccount } from "../../../wire";

export type Step = "up" | "down";

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
  readonly provider: string;
  readonly origin: string;
  readonly token: string | null;
}

// A list this editor sent, and the answer it was sent against, written
// out so that it compares by content (see `Filing` for why not by
// identity).
export interface Sent {
  readonly rows: readonly ProviderAccount[];
  readonly against: string;
}

function stamp(endpoint: EndpointSummary): string {
  return JSON.stringify(endpoint);
}

export interface Editor {
  draft: AccountDraft;
  // The id of the account the form is editing; null while adding.
  editing: string | null;
  note: string | null;
  refused: AxError | null;
  pending: Filing | null;
  sent: Sent | null;
}

// Where the city and pairing the page talks to stand right now.
export interface Reach {
  readonly origin: string;
  readonly token: string | null;
}

// Everything the wiring needs from outside: the endpoint as now
// answered, the language, the door to the city and to the vault, and
// the two places focus is put back.
export interface Hands {
  readonly endpoint: () => EndpointSummary;
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
  return { draft: BLANK, editing: null, note: null, refused: null, pending: null, sent: null };
}

const labelled = Schema.is(ServerLabel);

// The list the editor draws: the one it sent while that is newer than
// the city's answer, and the city's list otherwise. An endpoint
// attached with one legacy key lists none.
export function shown(editor: Editor, endpoint: EndpointSummary): readonly ProviderAccount[] {
  return editor.sent?.rows ?? endpoint.tuning.accounts ?? [];
}

// A newer answer for this endpoint replaces what was sent.
export function settle(editor: Editor, endpoint: EndpointSummary): void {
  if (editor.sent !== null && editor.sent.against !== stamp(endpoint)) editor.sent = null;
}

// The city refused the list: the editor draws the city's list again and
// says why beside it.
export function refuse(editor: Editor, error: AxError): void {
  editor.sent = null;
  editor.refused = error;
}

// Leaving the editor: a key still on its way lands nowhere.
export function leave(editor: Editor): void {
  editor.pending = null;
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
// one name `referenceFor` gives this provider and account, and the
// account carries the reference the city answers with.
export function save(editor: Editor, hands: Hands): void {
  const endpoint = hands.endpoint();
  if (unsaveable(editor, shown(editor, endpoint)) !== null) return;
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
  const filing: Filing = { ticket: Symbol("filing"), account, provider: endpoint.name, ...hands.reach() };
  const at = referenceFor(filing.provider, account.id);
  editor.pending = filing;
  void hands
    .enrol({ origin: filing.origin, token: filing.token, realm: at.realm, name: at.name, value: draft.key, lang: hands.lang() })
    .then((outcome) => {
      if (editor.pending?.ticket !== filing.ticket) return;
      editor.pending = null;
      const now = hands.reach();
      if (hands.endpoint().name !== filing.provider || now.origin !== filing.origin || now.token !== filing.token) return;
      if (outcome.kind === "stored") store(editor, hands, { ...filing.account, reference: outcome.reference });
      else editor.note = outcome.reason;
    });
}

// Puts one account into the list - in its own place when it is listed,
// last when it is new - and sends the list. The form empties only when
// the list went out.
function store(editor: Editor, hands: Hands, account: ProviderAccount): void {
  const rows = shown(editor, hands.endpoint());
  const at = rows.findIndex((row) => row.id === account.id);
  const next = at < 0 ? [...rows, account] : rows.map((row, index) => (index === at ? account : row));
  if (!commit(editor, hands, next)) return;
  editor.editing = null;
  editor.draft = BLANK;
}

export function move(editor: Editor, hands: Hands, id: string, step: Step): void {
  if (editor.pending !== null) return;
  const rows = shown(editor, hands.endpoint());
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
  const rows = shown(editor, hands.endpoint());
  if (rows.length <= 1) return;
  if (!commit(editor, hands, rows.filter((row) => row.id !== id))) return;
  if (editor.editing === id) {
    editor.editing = null;
    editor.draft = BLANK;
  }
  hands.focusHeading();
}

// Sends the whole list. A frame that could not leave keeps the draft
// and says so; one that left is drawn until the city answers.
function commit(editor: Editor, hands: Hands, rows: readonly ProviderAccount[]): boolean {
  const endpoint = hands.endpoint();
  if (!hands.send(reattachEndpoint(endpoint, rows))) {
    editor.note = say(hands.lang(), "setup_account_not_sent");
    return false;
  }
  editor.note = null;
  editor.refused = null;
  editor.sent = { rows, against: stamp(endpoint) };
  return true;
}
