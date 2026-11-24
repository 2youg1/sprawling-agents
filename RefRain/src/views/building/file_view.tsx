// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  Show,
  Switch,
  untrack,
} from "solid-js";
import { putSpine } from "../../core/commands";
import { readDocument } from "../../core/document";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kib } from "../../core/time";
import { ui } from "../../ui";
import { Address, type IdemKey, type Query, type SpineDocument } from "../../wire";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import Path from "../parts/path.solid";
import Unanswered from "../parts/unanswered.solid";
import Prose from "../prose";
import SourceToggle from "./source_toggle.look.solid";
import { sourceToggleWire } from "./source_toggle";
import TextBox from "./text_box.look.solid";
import { textBoxWire } from "./text_box";

const SPINE: Record<string, SpineDocument | undefined> = {
  "Roadmap.md": "roadmap",
  "Memo.md": "memo",
  "Handoff.md": "handoff",
  "SPEC.md": "spec",
};

interface Props {
  readonly at: Address;
  readonly root: Address;
}

interface Pending {
  readonly idem: IdemKey;
  readonly body: string;
}

function FileAt(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const receipts = storeValue(u.conn.receipts);
  const question = createMemo<Query>(() => ({ document: { at: props.at } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() => readDocument(asked()));
  const doc = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const missing = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const word = (key: Key) => say(lang(), key);
  const [raw, setRaw] = createSignal(false);
  const [base, setBase] = createSignal<string>();
  const [draft, setDraft] = createSignal("");
  const [pending, setPending] = createSignal<Pending>();
  const [errorKey, setErrorKey] = createSignal<IdemKey>();
  const markdown = () => props.at.endsWith(".md");

  const lastError = createMemo(() => {
    const key = errorKey();
    const receipt = key === undefined ? undefined : receipts().get(key);
    return receipt?.kind === "refused" ? receipt.error : undefined;
  });
  const spine = createMemo(() => {
    const cut = props.at.lastIndexOf("/");
    if (cut < 0) return null;
    const which = SPINE[props.at.slice(cut + 1)];
    return which === undefined ? null : { which, building: Address.make(props.at.slice(0, cut)) };
  });

  createEffect(
    () => ({ operation: pending(), actual: doc()?.text, receipts: receipts() }),
    ({ operation, actual, receipts }) => {
      if (operation === undefined) return;
      if (receipts.get(operation.idem)?.kind === "refused") {
        setErrorKey(operation.idem);
        setPending(undefined);
      } else if (actual === operation.body) {
        setPending(undefined);
        setBase(untrack(draft) === actual ? undefined : actual);
      }
    },
  );

  function change(): void {
    const held = doc();
    if (held === undefined || held.truncated || held.binary) return;
    setBase(held.text);
    setDraft(held.text);
    setErrorKey(undefined);
  }

  function save(): void {
    const to = spine();
    const from = base();
    const body = draft();
    if (to === null || from === undefined || body === from || pending() !== undefined) return;
    const command = putSpine(to.building, to.which, from, body);
    if (!("put_spine" in command) || !u.send(command)) return;
    setErrorKey(undefined);
    setPending({ idem: command.put_spine.idem, body });
    u.conn.asking.refresh(question());
  }

  const lines = createMemo(() => (doc()?.text ?? "").split("\n"));
  const gutter = () => `${String(String(lines().length).length + 1)}ch`;

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="flex items-center gap-base pb-snug font-mono text-note text-text-faint">
        <Path path={props.at} base={props.root} />
        <Show when={doc()}>
          {(held) => <span class="text-text-faint">{kib(held().bytes)}</span>}
        </Show>
        <span class="flex-1" />
        <Show when={markdown() && doc()?.binary === false}>
          <SourceToggle
            label=".md"
            pressed={raw()}
            wire={sourceToggleWire(raw(), () => {
              setRaw(!raw());
            })}
          />
          <Show when={spine() !== null && base() === undefined && doc()?.truncated === false}>
            <Button label={word("file_edit")} tone="quiet" onPress={change} />
          </Show>
        </Show>
      </div>
      <Switch>
        <Match when={read().kind === "asking"}>
          <p class="text-text-faint">…</p>
        </Match>
        <Match when={missing()}>
          {(unavailable) => (
            <Unanswered
              query={unavailable().query}
              asked={question()}
              reason={unavailable().reason}
            />
          )}
        </Match>
        <Match when={doc()}>
          {(stored) => (
            <div class="min-h-0 flex-1 overflow-auto rounded-panel bg-chrome/60 p-pane">
              <Show when={stored().binary}>
                <p class="text-text-faint">
                  {fill(word("file_binary"), { kib: kib(stored().bytes) })}
                </p>
              </Show>
              <Show when={stored().truncated}>
                <p class="mb-base text-note text-alert">
                  {fill(word("file_truncated"), {
                    kib: kib(stored().text.length),
                    total: kib(stored().bytes),
                  })}
                </p>
              </Show>
              <Show when={!stored().binary}>
                <Switch
                  fallback={
                    <ol class="overflow-x-auto font-mono text-note leading-read text-text-quiet">
                      <For each={lines()} keyed={false}>
                        {(line, index) => (
                          <li class="flex whitespace-pre">
                            <span
                              class="shrink-0 select-none pr-base text-right text-text-faint"
                              style={{ width: gutter() }}
                            >
                              {index + 1}
                            </span>
                            <span>{line()}</span>
                          </li>
                        )}
                      </For>
                    </ol>
                  }
                >
                  <Match when={base() !== undefined}>
                    <TextBox
                      size="document"
                      wire={textBoxWire(draft(), props.at, false, setDraft)}
                    />
                    <div class="mt-base flex gap-base">
                      <Button
                        label={word("file_save")}
                        onPress={save}
                        loading={pending() !== undefined}
                      />
                      <Button
                        label={word("file_discard")}
                        tone="quiet"
                        onPress={() => {
                          if (pending() === undefined) setBase(undefined);
                        }}
                      />
                    </div>
                    <Show when={lastError()}>
                      {(error) => <DiagnosticView error={error()} lang={lang()} />}
                    </Show>
                  </Match>
                  <Match when={markdown() && !raw()}>
                    <div class="mx-auto w-full max-w-measure">
                      <Prose text={stored().text} />
                    </div>
                  </Match>
                </Switch>
              </Show>
            </div>
          )}
        </Match>
      </Switch>
    </div>
  );
}

export default function FileView(props: Props) {
  return (
    <Show when={props.at} keyed>
      {(at) => <FileAt at={at} root={props.root} />}
    </Show>
  );
}
