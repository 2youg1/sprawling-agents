// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, Show, untrack } from "solid-js";

import { putDocument } from "../../core/commands";
import { readDocument } from "../../core/document";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import {
  Address,
  type AxError,
  type Command,
  type GovernedDocument,
  type IdemKey,
  type Query,
} from "../../wire";
import Button from "../parts/button.solid";
import Segmented from "../parts/segmented.solid";
import Unanswered from "../parts/unanswered.solid";
import SaveCard from "../settings/save_card_view";
import { edited, RECEIPT_MS, sent, waited, type Saving } from "../settings/saving";
import { draftOf } from "./draft";
import Draft from "./draft.look.solid";

const FILES: Record<GovernedDocument, string> = {
  mayor: "MAYOR.md",
  clerk: "CLERK.md",
  preferences: "PREFERENCES.md",
};

const DOCUMENTS: readonly GovernedDocument[] = ["mayor", "clerk", "preferences"];

function governedAt(which: GovernedDocument): Address {
  return Address.make(`.sprawling/${FILES[which]}`);
}

interface Write {
  readonly idem: IdemKey;
  readonly body: string;
  readonly started: number;
}

interface Change {
  readonly base: string;
  readonly body: string;
  readonly write?: Write;
  readonly uncertain?: boolean;
  readonly error?: AxError;
}

interface DocumentDraftProps<T extends string> {
  readonly which: T;
  readonly path: (which: T) => Address;
  readonly command: (
    at: T,
    base: string,
    text: string,
  ) => { readonly frame: Command; readonly idem: IdemKey } | undefined;
  readonly title: string;
  readonly note: string;
  readonly field: Key;
  readonly save: Key;
  readonly children: JSX.Element;
}

export function DocumentDraft<T extends string>(props: DocumentDraftProps<T>) {
  const u = ui();
  const lang = storeValue(u.lang);
  const receipts = storeValue(u.conn.receipts);
  const which = () => props.which;
  const [changes, setChanges] = createSignal<ReadonlyMap<T, Change>>(new Map());
  const [lastSaved, setLastSaved] = createSignal<T>();
  const query = createMemo<Query>(() => ({ document: { at: props.path(which()) } }));
  const answer = storeValue(() => u.conn.asking.ask(query()));
  const read = createMemo(() => readDocument(answer()));
  const onDisk = createMemo(() => {
    const stored = read();
    if (stored.kind === "held" && !stored.value.binary && !stored.value.truncated)
      return stored.value.text;
    const response = answer();
    return response !== undefined && "document" in response && response.document.state === "missing"
      ? ""
      : undefined;
  });
  const change = () => changes().get(which());
  const write = () => change()?.write;
  const body = () => change()?.body ?? onDisk() ?? "";
  const word = (key: Key) => say(lang(), key);
  const refresh = () => u.conn.asking.refresh(query());

  function keep(at: T, value: Change | undefined): void {
    setChanges((before) => {
      const next = new Map(before);
      if (value === undefined) next.delete(at);
      else next.set(at, value);
      return next;
    });
  }

  function edit(text: string): void {
    const at = which();
    const edited = change();
    const base = edited?.base ?? onDisk();
    if (base === undefined) return;
    setLastSaved(undefined);
    if (edited?.write !== undefined || edited?.error?.code === "E_VERSION_CONFLICT") {
      keep(at, { ...edited, body: text });
    } else {
      keep(at, text === base ? undefined : { base, body: text });
    }
  }

  function confirmCurrentBase(): void {
    const local = change();
    const actual = onDisk();
    if (local?.error?.code !== "E_VERSION_CONFLICT" || actual === undefined) return;
    keep(which(), local.body === actual ? undefined : { base: actual, body: local.body });
  }

  function save(): void {
    const at = which();
    const local = change();
    const actual = onDisk();
    if (local === undefined || local.write !== undefined || actual === undefined) return;
    if (local.error?.code === "E_VERSION_CONFLICT") return;
    const base = local.base;
    const command = props.command(at, base, local.body);
    if (command === undefined || !u.send(command.frame)) return;
    keep(at, {
      base,
      body: local.body,
      write: {
        idem: command.idem,
        body: local.body,
        started: u.now(),
      },
    });
    refresh();
  }

  const status = createMemo<Saving>(() => {
    const local = change();
    if (local?.write !== undefined) {
      const sentAt = sent(local.base);
      return local.uncertain === true ? waited(sentAt) : sentAt;
    }
    if (local?.error !== undefined) return { kind: "refused", error: local.error };
    return lastSaved() === which() && local === undefined
      ? { kind: "saved" }
      : edited(local !== undefined);
  });

  createEffect(
    () => ({ at: which(), actual: onDisk(), operation: write(), receipts: receipts() }),
    ({ at, actual, operation, receipts }) => {
      if (operation === undefined) return;
      const local = untrack(changes).get(at);
      if (local === undefined || local.write !== operation) return;
      const outcome = receipts.get(operation.idem);
      if (outcome?.kind === "refused") {
        keep(at, { base: local.base, body: local.body, error: outcome.error });
        refresh();
        return;
      }
      if (actual === operation.body) {
        setLastSaved(() => at);
        keep(at, local.body === actual ? undefined : { base: actual, body: local.body });
      }
    },
  );

  createEffect(
    () => ({ at: which(), question: query(), write: write() }),
    ({ at, question, write }) => {
      if (write === undefined) return;
      const recheck = () => u.conn.asking.refresh(question);
      const poll = window.setInterval(recheck, 1_000);
      const timeout = window.setTimeout(
        () => {
          const local = changes().get(at);
          if (local?.write === write) keep(at, { ...local, uncertain: true });
          window.clearInterval(poll);
        },
        Math.max(0, RECEIPT_MS - (u.now() - write.started)),
      );
      return () => {
        window.clearTimeout(timeout);
        window.clearInterval(poll);
      };
    },
  );

  const conflict = () => change()?.error?.code === "E_VERSION_CONFLICT";
  const missing = () => {
    const result = read();
    return result.kind === "unavailable" ? result : undefined;
  };
  return (
    <SaveCard
      title={props.title}
      note={props.note}
      settled={word("settings_next_run")}
      saveLabel={word(props.save)}
      saving={status()}
      blocked={
        conflict()
          ? "setup_governed_compare"
          : onDisk() === undefined
            ? "setup_governed_unchanged"
            : undefined
      }
      onSave={save}
      onCheck={refresh}
    >
      {props.children}
      <Draft {...draftOf(word(props.field), body(), edit)} />
      <Show when={missing()}>
        {(result) => <Unanswered query={result().query} asked={query()} reason={result().reason} />}
      </Show>
      <Show when={conflict() && onDisk() !== undefined}>
        <details>
          <summary>{word("setup_governed_compare")}</summary>
          <pre class="max-h-output overflow-auto whitespace-pre-wrap">{onDisk()}</pre>
          <Button label={word("setup_governed_rebase")} onPress={confirmCurrentBase} />
        </details>
      </Show>
      <code class="truncate font-mono text-note text-text-faint">{props.path(which())}</code>
    </SaveCard>
  );
}

export default function GovernedSection() {
  const lang = storeValue(ui().lang);
  const [which, setWhich] = createSignal<GovernedDocument>("mayor");
  return (
    <DocumentDraft
      which={which()}
      path={governedAt}
      command={(at, base, text) => {
        const frame = putDocument(at, base, text);
        return "put_document" in frame ? { frame, idem: frame.put_document.idem } : undefined;
      }}
      title={say(lang(), "setup_governed")}
      note={say(lang(), "setup_governed_note")}
      field={`governed_${which()}`}
      save="setup_governed_save"
    >
      <Segmented
        label={say(lang(), "setup_governed")}
        options={DOCUMENTS.map((each) => ({
          value: each,
          label: say(lang(), `governed_${each}`),
        }))}
        held={which()}
        onPick={setWhich}
      />
    </DocumentDraft>
  );
}
