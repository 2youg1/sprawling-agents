// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the run produced, beside the conversation that produced it.
//
// **Driven by the ledger, not by the address bar.** The card shows the
// newest thing this run touched of each kind, all read from the same
// rounds the thread beside it is drawn from. A route of its own would
// add a second answer to "which run am I looking at", and the two
// answers would differ the first time a person opened a link.
//
// **One card, three panes - not three cards.** A tool result is one of
// three things to read, and they are not peers on the page: a file
// somebody read is the subject and takes the body, while what changed
// and what a command printed are two readings of the same run and take
// one strip at the foot of the card, one at a time. Three cards would
// make the eye choose which to look at first; one card with a head, a
// body and a tabbed foot says which is the subject and keeps the other
// two a click away.
//
// **The three are the three deeds `trace.ts` already names.** Reading,
// changing and running are told apart by `deedOf`, which is the same
// authority the counting sentence above the thread uses. Nothing here
// classifies a call for itself.
//
// **The control that opens and closes this card is not in it.** It sits
// beside the thread, where it is reachable when the card is closed; a
// second one in the head would be a second place to look for one piece
// of state, and the closed case would still need the first. The
// reference this card is measured against puts a cross in the head
// because its panel cannot be closed from anywhere else.

import { Show, createMemo, createSignal } from "solid-js";

import type { Call } from "../../wire";
import { useSay } from "../../ui";
import { Code } from "../parts/code";
import { Tabs } from "../parts/tabs";
import type { Lens } from "../parts/tabs";
import { callWord } from "./calls";
import type { Artifacts } from "./trace";

// The name the toggle points at with `aria-controls`. Written once so
// the control and the region it opens cannot drift apart.
export const PANEL_ID = "artifact";

// The two readings the foot of the card holds. A closed set, because
// the tab strip is built from it and a third reading would have to be
// a pane rather than a string somebody added.
type Foot = "wrote" | "terminal";

function head(call: Call): string {
  return call.output?.head ?? "";
}

function cut(call: Call): number {
  return call.output?.cut ?? 0;
}

// What a pane shows when its call was answered with more than the wire
// would carry.
function Cut(props: { readonly call: Call }) {
  const say = useSay();
  return (
    <Show when={cut(props.call) > 0}>
      <p class="border-t border-edge px-snug py-tight text-note text-text-disabled">
        {say("run_cut", { n: String(cut(props.call)) })}
      </p>
    </Show>
  );
}

// A pane of plain output: what a command printed, or what an edit
// reported. Monospaced, scrolling inside itself, and sitting on the
// page rather than on a fill - the deepest surface on the screen is
// the content, which is what the head and the strips around it lift
// off.
function Printed(props: { readonly call: Call; readonly label: string }) {
  return (
    <section class="flex min-h-0 flex-1 flex-col bg-page" aria-label={props.label}>
      <p class="truncate border-b border-edge px-snug py-tight font-mono text-note text-text-faint">
        {callWord(props.call.tool, props.call.subject)}
      </p>
      <pre class="min-h-0 flex-1 overflow-auto px-snug py-tight font-mono text-note leading-relaxed text-text-quiet">
        {head(props.call)}
      </pre>
      <Cut call={props.call} />
    </section>
  );
}

export interface ArtifactProps {
  readonly artifacts: Artifacts;
  // Whether the person has the card open. Drawn either way rather than
  // mounted on demand, so the toggle beside it always names a region
  // that exists.
  readonly open: boolean;
}

export function Artifact(props: ArtifactProps) {
  const say = useSay();
  const read = createMemo(() => props.artifacts.read);
  const wrote = createMemo(() => props.artifacts.wrote);
  const terminal = createMemo(() => props.artifacts.terminal);

  // Which readings the foot can offer, in the order they are drawn.
  const offered = createMemo<readonly Foot[]>(() => {
    const held: Foot[] = [];
    if (terminal() !== null) held.push("terminal");
    if (wrote() !== null) held.push("wrote");
    return held;
  });
  const [picked, setPicked] = createSignal<Foot | null>(null);
  // The person's choice while it is still on offer, and the first
  // reading there is otherwise. Derived rather than corrected by an
  // effect: a run that stops producing command output must not leave
  // the card pointing at a pane that is no longer there.
  const showing = createMemo<Foot | null>(() => {
    const held = picked();
    const there = offered();
    return held !== null && there.includes(held) ? held : (there[0] ?? null);
  });
  const lenses = createMemo<readonly Lens[]>(() =>
    offered().map((foot) => ({
      id: foot,
      label: foot === "terminal" ? say("talk_panel_terminal") : say("talk_panel_change"),
    })),
  );

  return (
    <aside
      id={PANEL_ID}
      aria-label={say("talk_panel")}
      class={`flex max-h-output min-h-0 w-full flex-col overflow-hidden border-t border-edge bg-chrome @lg/page:max-h-none @lg/page:w-tree @lg/page:shrink-0 @lg/page:border-t-0 @lg/page:border-l @wide/page:max-w-measure @wide/page:flex-1 ${
        props.open ? "" : "hidden"
      }`}
    >
      {/* The head names the subject of the body, and takes the shell's
          one bar height rather than a second nearly-equal number of
          its own. */}
      <Show when={read() ?? wrote() ?? terminal()}>
        {(subject) => (
          <header class="flex h-bar shrink-0 items-center gap-snug border-b border-edge px-snug">
            <span class="min-w-0 flex-1 truncate font-mono text-label text-text">
              {subject().subject ?? subject().tool}
            </span>
          </header>
        )}
      </Show>

      <Show when={read()}>
        {(call) => (
          <section
            class="flex min-h-0 flex-1 flex-col border-b border-edge bg-page"
            aria-label={say("talk_panel_code")}
          >
            <Code path={call().subject ?? ""} text={head(call())} />
            <Cut call={call()} />
          </section>
        )}
      </Show>

      <Show when={showing()}>
        {(foot) => (
          <div class="flex min-h-0 flex-1 shrink-0 flex-col">
            {/* One reading is not a choice, so no strip is drawn for
                it: the pane names itself in its own first line. */}
            <Show when={lenses().length > 1}>
              <Tabs
                label={say("talk_panel")}
                lenses={lenses()}
                current={foot()}
                onPick={(id) => {
                  setPicked(id === "terminal" ? "terminal" : "wrote");
                }}
              />
            </Show>
            <Show
              when={foot() === "terminal" ? terminal() : wrote()}
              keyed
            >
              {(call) => (
                <Printed
                  call={call}
                  label={foot() === "terminal" ? say("talk_panel_terminal") : say("talk_panel_change")}
                />
              )}
            </Show>
          </div>
        )}
      </Show>
    </aside>
  );
}
