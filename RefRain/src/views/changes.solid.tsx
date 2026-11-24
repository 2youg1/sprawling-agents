// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";
import { readAnswer } from "../core/answered";
import { say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Address, GitOid, Query } from "../wire";
import { howWord, linesWord } from "./changes";
import Patch from "./inspect/patch.solid";
import Glyph from "./parts/glyph.solid";
import Unanswered from "./parts/unanswered.solid";

export interface ChangesProps {
  readonly base: GitOid;
  readonly head: GitOid | null;
  readonly talk?: Address | undefined;
  readonly act?: ((path: string) => JSX.Element) | undefined;
}

export default function Changes(props: ChangesProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const [opened, setOpened] = createSignal<string | null>(null);
  const question = createMemo<Query>(() => ({ changes: { base: props.base, head: props.head } }));
  const answer = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(answer(), (held) => ("changes" in held ? held.changes.files : undefined)),
  );
  const refusal = () => {
    const held = read();
    return held.kind === "unavailable" ? held.query : undefined;
  };
  const files = () => {
    const held = read();
    return held.kind === "held" ? held.value : undefined;
  };
  const patchQuestion = createMemo<Query | undefined>(() => {
    const path = opened();
    const head = props.head;
    return path === null || head === null
      ? undefined
      : { hunks: { oid_a: props.base, oid_b: head, path } };
  });
  const patchAnswer = storeValue(() => {
    const query = patchQuestion();
    return query === undefined ? undefined : u.conn.asking.ask(query);
  });
  const patchRead = createMemo(() =>
    readAnswer(patchAnswer(), (held) => ("hunks" in held ? held.hunks : undefined)),
  );
  const rejected = () => {
    const item = patchRead();
    const query = patchQuestion();
    return item.kind === "unavailable" && query !== undefined
      ? { query, name: item.query }
      : undefined;
  };
  const patch = () => {
    const item = patchRead();
    return item.kind === "held" ? item.value : undefined;
  };

  return (
    <Switch fallback={<p class="text-text-faint">…</p>}>
      <Match when={refusal()}>{(kind) => <Unanswered query={kind()} asked={question()} />}</Match>
      <Match when={files()?.length === 0}>
        <p class="text-text-faint">{say(lang(), "run_no_changes")}</p>
      </Match>
      <Match when={files()}>
        {(rows) => (
          <ul class="text-note">
            <For each={rows()} keyed={(file) => file.path}>
              {(file) => {
                const open = () => opened() === file().path;
                return (
                  <li class="border-b border-edge">
                    <div class="flex items-center gap-snug">
                      <button
                        type="button"
                        data-control="surface"
                        aria-expanded={open() ? "true" : "false"}
                        class="flex min-h-control-sm min-w-0 flex-1 items-center gap-snug rounded-control px-snug text-left"
                        onClick={() => {
                          const path = file().path;
                          setOpened((last) => (last === path ? null : path));
                        }}
                      >
                        <Glyph name="chevron" size="sm" class={open() ? "rotate-90" : ""} />
                        <span class="truncate text-text-faint">{howWord(lang(), file().how)}</span>
                        <span class="truncate font-mono text-text-quiet">{file().path}</span>
                        <span class="font-mono text-text-faint">
                          {linesWord(lang(), file().lines)}
                        </span>
                      </button>
                      {props.act?.(file().path)}
                    </div>
                    <Show when={open()}>
                      <Switch fallback={<p class="text-text-faint">…</p>}>
                        <Match when={props.head === null}>
                          <p class="pb-base text-text-faint">
                            {say(lang(), "run_patch_needs_checkpoint")}
                          </p>
                        </Match>
                        <Match when={rejected()}>
                          {(error) => <Unanswered query={error().name} asked={error().query} />}
                        </Match>
                        <Match when={patch()}>
                          {(value) => (
                            <div class="pb-base">
                              <div class="overflow-x-auto rounded-card border border-edge bg-page">
                                <Patch patch={value()} talk={props.talk} />
                              </div>
                            </div>
                          )}
                        </Match>
                      </Switch>
                    </Show>
                  </li>
                );
              }}
            </For>
          </ul>
        )}
      </Match>
    </Switch>
  );
}
