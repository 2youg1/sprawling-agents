// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a turn did, folded to one line.
//
// A wave of tool calls is the bulk of what a run produces and almost
// none of what a person reads a thread for, so the thread states the
// shape of the wave - explored so many files, wrote so many, ran so
// many commands - and opens to the calls themselves on a click.
//
// **The fold is a `<details>`.** Opening, closing, the disclosure
// triangle, the keyboard and the state a screen reader reports are the
// element's, and the height travels because `theme.css` declares
// `interpolate-size` once for the whole page. An engine that has
// neither opens the fold at once, which is the behaviour that engine
// has today and not a branch anybody writes.

import { For, Show, createMemo } from "solid-js";

import { toFragment } from "../../core/route";
import { count } from "../../core/time";
import type { Call, RunId } from "../../wire";
import { useSay } from "../../ui";
import { tally } from "./trace";

// How a call is named wherever one is named: in the fold below, and in
// the posture line while the call is still running. One spelling, so a
// person does not have to recognise the same call twice.
export function callWord(tool: string, subject: string | null | undefined): string {
  return subject === null || subject === undefined || subject === "" ? tool : `${tool} ${subject}`;
}

export interface CallsProps {
  readonly calls: readonly Call[];
  readonly run: RunId;
}

export function Calls(props: CallsProps) {
  const say = useSay();
  // One clause per class of work that happened, and none for a class
  // that did not: "explored 0 files" is a sentence about nothing.
  const summary = createMemo(() => {
    const counted = tally(props.calls);
    const said: string[] = [];
    if (counted.explored > 0) said.push(say("talk_deed_explored", { n: count(counted.explored) }));
    if (counted.wrote > 0) said.push(say("talk_deed_wrote", { n: count(counted.wrote) }));
    if (counted.ran > 0) said.push(say("talk_deed_ran", { n: count(counted.ran) }));
    if (counted.other > 0) said.push(say("talk_deed_other", { n: count(counted.other) }));
    return said.join(" · ");
  });
  return (
    <details class="my-tight text-note text-text-faint">
      <summary class="cursor-pointer rounded-control px-tight marker:text-text-disabled hover:bg-chrome hover:text-text-quiet">
        {summary()}
      </summary>
      <ul class="mt-tight ml-pane border-l border-edge pl-base">
        <For each={props.calls}>
          {(call) => (
            <li class="my-tight">
              <span
                class={
                  call.outcome === "failed"
                    ? "text-alert"
                    : call.outcome === "waiting"
                      ? "animate-pulse"
                      : ""
                }
              >
                {callWord(call.tool, call.subject)}
              </span>
              <Show when={call.output}>
                {(output) => (
                  <>
                    <pre class="mt-tight max-h-output overflow-auto rounded-card border border-edge bg-page p-snug font-mono text-note text-text-quiet">
                      {output().head}
                      <Show when={output().cut > 0}>
                        {"\n"}
                        <span class="text-text-disabled">{say("run_cut", { n: String(output().cut) })}</span>
                      </Show>
                    </pre>
                    <Show when={output().cut > 0}>
                      <a
                        class="text-note text-text-faint hover:text-text-quiet"
                        href={toFragment({ kind: "run", run: props.run })}
                      >
                        {say("talk_call_open")}
                      </a>
                    </Show>
                  </>
                )}
              </Show>
            </li>
          )}
        </For>
      </ul>
    </details>
  );
}
