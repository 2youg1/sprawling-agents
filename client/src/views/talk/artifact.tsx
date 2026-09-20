// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the run produced, beside the conversation that produced it.
//
// **Driven by the ledger, not by the address bar.** The panel shows the
// newest file this run touched and the newest command it ran, both read
// from the same rounds the thread beside it is drawn from. A route of
// its own would add a second answer to "which run am I looking at", and
// the two answers would differ the first time a person opened a link.
//
// Two halves because a tool result is one of two things to read: a file,
// which wants line numbers and a name, and what a command printed, which
// wants neither. A run that only ran commands gets the lower half alone.

import { Show, createMemo } from "solid-js";

import type { Call } from "../../wire";
import { useSay } from "../../ui";
import { Code } from "../parts/code";
import { callWord } from "./calls";
import type { Artifacts } from "./trace";

// The name the toggle points at with `aria-controls`. Written once so
// the control and the region it opens cannot drift apart.
export const PANEL_ID = "artifact";

function head(call: Call): string {
  return call.output?.head ?? "";
}

function cut(call: Call): number {
  return call.output?.cut ?? 0;
}

export interface ArtifactProps {
  readonly artifacts: Artifacts;
  // Whether the person has the panel open. Drawn either way rather
  // than mounted on demand, so the toggle beside it always names a
  // region that exists.
  readonly open: boolean;
}

export function Artifact(props: ArtifactProps) {
  const say = useSay();
  const file = createMemo(() => props.artifacts.file);
  const terminal = createMemo(() => props.artifacts.terminal);
  return (
    <aside
      id={PANEL_ID}
      aria-label={say("talk_panel")}
      class={`flex max-h-output min-h-0 w-full flex-col overflow-hidden border-t border-g2 bg-g1 @lg/page:max-h-none @lg/page:w-tree @lg/page:shrink-0 @lg/page:border-t-0 @lg/page:border-l @wide/page:max-w-measure @wide/page:flex-1 ${
        props.open ? "" : "hidden"
      }`}
    >
      <Show when={file()}>
        {(call) => (
          <section
            class="flex min-h-0 flex-1 flex-col border-b border-g2"
            aria-label={say("talk_panel_code")}
          >
            <Code path={call().subject ?? ""} text={head(call())} />
            <Show when={cut(call()) > 0}>
              <p class="border-t border-g2 px-snug py-tight text-note text-text-disabled">
                {say("run_cut", { n: String(cut(call())) })}
              </p>
            </Show>
          </section>
        )}
      </Show>
      <Show when={terminal()}>
        {(call) => (
          <section class="flex min-h-0 flex-1 flex-col" aria-label={say("talk_panel_terminal")}>
            <p class="truncate border-b border-g2 px-snug py-tight font-mono text-note text-text-faint">
              {callWord(call().tool, call().subject)}
            </p>
            <pre class="min-h-0 flex-1 overflow-auto px-snug py-tight font-mono text-note leading-relaxed text-text-quiet">
              {head(call())}
              <Show when={cut(call()) > 0}>
                {"\n"}
                <span class="text-text-disabled">{say("run_cut", { n: String(cut(call())) })}</span>
              </Show>
            </pre>
          </section>
        )}
      </Show>
    </aside>
  );
}
