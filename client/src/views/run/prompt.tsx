// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this run was told, in the four segments it was sent as. Each one
// folds open, states the files it was read from and what the budget cut
// off the end of them, and copies as the text the model saw.
//
// The bytes come from the store, addressed by the hash the ledger
// recorded; a segment the store no longer holds says so rather than
// showing an empty box a reader would take for an empty prompt.

import { For, Show, createMemo, createSignal } from "solid-js";

import { buildingOf } from "../../core/route";
import { count } from "../../core/time";
import type { PrefixSegment, RunId } from "../../wire";
import { useGo, useSay, useUi } from "../../ui";
import { Path } from "../parts/path";

function Segment(props: { readonly segment: PrefixSegment }) {
  const say = useSay();
  const go = useGo();
  const [open, setOpen] = createSignal(false);
  const [copied, setCopied] = createSignal(false);
  const copy = () => {
    void navigator.clipboard.writeText(props.segment.text);
    setCopied(true);
  };
  return (
    <li class="border-b border-g1">
      <div class="flex items-center gap-base py-snug text-note">
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-base text-left hover:text-text"
          aria-expanded={open()}
          onClick={() => {
            setOpen((held) => !held);
          }}
        >
          <span
            class={`w-base shrink-0 text-center text-text-disabled transition-transform ${open() ? "rotate-90" : ""}`}
            aria-hidden="true"
          >
            ›
          </span>
          <span class="shrink-0 text-text-quiet">{say(`slot_${props.segment.slot}`)}</span>
          <span class="shrink-0 text-text-disabled">
            {say("run_prompt_bytes", { n: count(props.segment.bytes) })}
          </span>
          <Show when={!props.segment.stored}>
            <span class="truncate text-alert">{say("run_prompt_gone")}</span>
          </Show>
        </button>
        <Show when={props.segment.stored}>
          <button
            type="button"
            class="shrink-0 rounded-control bg-g1 px-snug py-tight text-label text-text-quiet hover:bg-g2 hover:text-text"
            onClick={copy}
          >
            {copied() ? say("run_prompt_copied") : say("run_prompt_copy")}
          </button>
        </Show>
      </div>
      <Show when={open()}>
        <div class="pb-base pl-wide">
          <Show when={props.segment.sources.length > 0}>
            <p class="flex flex-wrap items-baseline gap-snug pb-snug text-note text-text-faint">
              <span>{say("run_prompt_sources")}</span>
              <For each={props.segment.sources}>
                {(source) => (
                  <>
                    <Path
                      path={source.addr}
                      onOpen={() => {
                        go({ kind: "building", address: buildingOf(source.addr) });
                      }}
                    />
                    <Show when={source.dropped > 0}>
                      <span class="text-alert">
                        {say("run_prompt_dropped", { n: count(source.dropped) })}
                      </span>
                    </Show>
                  </>
                )}
              </For>
            </p>
          </Show>
          <pre class="overflow-x-auto whitespace-pre-wrap break-words rounded-control bg-g1 p-base font-mono text-note text-text-quiet">
            {props.segment.text}
          </pre>
        </div>
      </Show>
    </li>
  );
}

export function Prompt(props: { readonly run: RunId }) {
  const ui = useUi();
  const say = useSay();
  const answer = createMemo(() => ui.conn.asking.ask({ prefix: { run: props.run } }));
  const segments = createMemo(() => {
    const held = answer()();
    if (held === undefined) return undefined;
    return "prefix" in held ? held.prefix.segments : [];
  });
  return (
    <Show when={segments()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => (
        <Show when={held().length > 0} fallback={<p class="text-text-faint">{say("run_no_prompt")}</p>}>
          <ul>
            <For each={held()}>{(segment) => <Segment segment={segment} />}</For>
          </ul>
        </Show>
      )}
    </Show>
  );
}
