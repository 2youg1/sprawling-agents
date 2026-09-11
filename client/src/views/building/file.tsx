// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A file as it is on disk: Markdown set as prose unless the reader asks
// for the source, everything else numbered line by line. The head of the
// file says where it is, and says how much was left out when the answer
// was cut short.

import { For, Show, createMemo, createSignal } from "solid-js";

import { kib } from "../../core/time";
import type { Address } from "../../wire";
import { useSay, useUi } from "../../ui";
import { Path } from "../parts/path";
import { Prose } from "../prose";

function Lines(props: { readonly text: string }) {
  const lines = createMemo(() => props.text.split("\n"));
  const width = () => `${String(String(lines().length).length + 1)}ch`;
  return (
    <ol class="overflow-x-auto font-mono text-note leading-relaxed text-text-quiet">
      <For each={lines()}>
        {(line, index) => (
          <li class="flex whitespace-pre">
            <span class="shrink-0 select-none pr-base text-right text-text-disabled" style={{ width: width() }}>
              {index() + 1}
            </span>
            <span>{line}</span>
          </li>
        )}
      </For>
    </ol>
  );
}

export function FileView(props: { readonly at: Address; readonly root: Address }) {
  const ui = useUi();
  const say = useSay();
  const document = createMemo(() => ui.conn.asking.ask({ document: { at: props.at } }));
  const doc = createMemo(() => {
    const answer = document()();
    if (answer === undefined) return undefined;
    return "document" in answer ? answer.document : null;
  });
  const markdown = () => props.at.endsWith(".md");
  const [raw, setRaw] = createSignal(false);
  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="flex items-center gap-base pb-snug font-mono text-note text-text-faint">
        <Path path={props.at} base={props.root} />
        <Show when={doc()}>{(held) => <span class="text-text-disabled">{kib(held().bytes)}</span>}</Show>
        <span class="flex-1" />
        <Show when={markdown() && doc()?.binary === false}>
          <button
            type="button"
            class={`rounded-pill px-snug text-note ${raw() ? "bg-g2 text-text" : "text-text-disabled hover:text-text-quiet"}`}
            onClick={() => setRaw((held) => !held)}
          >
            .md
          </button>
        </Show>
      </div>
      <Show when={doc()} fallback={<p class="text-text-disabled">{doc() === null ? say("file_missing") : "…"}</p>}>
        {(held) => (
          <div class="min-h-0 flex-1 overflow-auto rounded-panel bg-g1/60 p-pane">
            <Show when={held().binary}>
              <p class="text-text-faint">{say("file_binary", { kib: kib(held().bytes) })}</p>
            </Show>
            <Show when={held().truncated}>
              <p class="mb-base text-note text-alert">
                {say("file_truncated", { kib: kib(held().text.length), total: kib(held().bytes) })}
              </p>
            </Show>
            <Show when={!held().binary}>
              <Show when={markdown() && !raw()} fallback={<Lines text={held().text} />}>
                <Prose text={held().text} />
              </Show>
            </Show>
          </div>
        )}
      </Show>
    </div>
  );
}
