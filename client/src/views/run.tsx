// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One run under four lenses: what was said (turns), what it has seen
// and how full its window is (context), what moved on disk (changes),
// and what it left to be checked (evidence). The conversation is the
// same component the first page draws, so a run reads the same from
// both doors.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { sendingInto } from "../core/belief";
import { cancel, steer } from "../core/commands";
import { buildingOf, roomOf, toFragment } from "../core/route";
import { count, usd } from "../core/time";
import type { GitOid, RoundsAnswer, RunId, Turn } from "../wire";
import { useCommand, useHearing, useSay, useUi } from "../ui";
import { Changes } from "./changes";
import { Composer } from "./talk/composer";
import { Thread } from "./talk/thread";

export interface RunProps {
  readonly run: RunId;
}

type Lens = "turns" | "context" | "changes" | "evidence";
const LENSES: readonly Lens[] = ["turns", "context", "changes", "evidence"];

function Context(props: { readonly rounds: RoundsAnswer }) {
  const say = useSay();
  const peak = createMemo(() =>
    Math.max(1, ...props.rounds.turns.map((turn) => (turn.used?.input ?? 0) + (turn.used?.output ?? 0))),
  );
  const seen = createMemo(() => {
    const files = new Map<string, number>();
    for (const turn of props.rounds.turns) {
      for (const call of turn.calls) {
        if ((call.tool === "read" || call.tool === "search") && call.subject) {
          files.set(call.subject, (files.get(call.subject) ?? 0) + 1);
        }
      }
    }
    return [...files.entries()].sort((a, b) => b[1] - a[1]);
  });
  const spent = createMemo(() => props.rounds.turns.reduce((sum, turn) => sum + (turn.spent ?? 0), 0));
  const bar = (turn: Turn, part: "input" | "cached" | "output") => {
    const used = turn.used;
    if (used === null || used === undefined) return 0;
    const value = part === "cached" ? Math.min(used.cached, used.input) : part === "input" ? used.input - Math.min(used.cached, used.input) : used.output;
    return (value / peak()) * 100;
  };
  return (
    <div class="grid gap-wide md:grid-cols-2">
      <section>
        <h2 class="mb-base text-label font-label text-text-quiet">{say("run_window")}</h2>
        <Show when={props.rounds.turns.some((turn) => turn.used)} fallback={<p class="text-text-faint">{say("run_no_usage")}</p>}>
          <ul class="text-note">
            <For each={props.rounds.turns}>
              {(turn) => (
                <li class="my-tight flex items-center gap-snug">
                  <span class="w-figure shrink-0 text-text-disabled">{say("run_turn_n", { n: String(turn.number) })}</span>
                  <span class="flex h-dot flex-1 overflow-hidden rounded-pill bg-g1">
                    <span class="bg-g5" style={{ width: `${String(bar(turn, "cached"))}%` }} title={say("run_cached")} />
                    <span class="bg-accent" style={{ width: `${String(bar(turn, "input"))}%` }} title={say("run_input")} />
                    <span class="bg-accent-solid" style={{ width: `${String(bar(turn, "output"))}%` }} title={say("run_output")} />
                  </span>
                  <span class="w-figure shrink-0 text-right text-text-faint">
                    {turn.used ? count(turn.used.input + turn.used.output) : "—"}
                  </span>
                </li>
              )}
            </For>
          </ul>
          <p class="mt-snug text-note text-text-disabled">
            <span class="mr-base"><span class="inline-block size-dot rounded-pill bg-g5" /> {say("run_cached")}</span>
            <span class="mr-base"><span class="inline-block size-dot rounded-pill bg-accent" /> {say("run_input")}</span>
            <span><span class="inline-block size-dot rounded-pill bg-accent-solid" /> {say("run_output")}</span>
          </p>
        </Show>
        <Show when={spent() > 0}>
          <p class="mt-base text-note text-text-quiet">{say("run_spent", { usd: usd(spent()) })}</p>
        </Show>
      </section>
      <section>
        <h2 class="mb-base text-label font-label text-text-quiet">{say("run_read_files")}</h2>
        <Show when={seen().length > 0} fallback={<p class="text-text-faint">{say("run_read_nothing")}</p>}>
          <ul class="text-note">
            <For each={seen()}>
              {([file, n]) => (
                <li class="my-tight flex justify-between gap-base">
                  <span class="truncate font-mono text-text-quiet">{file}</span>
                  <span class="text-text-disabled">{n > 1 ? `×${String(n)}` : ""}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </div>
  );
}

function Evidence(props: { readonly run: RunId }) {
  const ui = useUi();
  const say = useSay();
  const evidence = createMemo(() => ui.conn.asking.ask({ evidence: { run: props.run } }));
  const items = createMemo(() => {
    const held = evidence()();
    return held !== undefined && "evidence" in held ? held.evidence.items : undefined;
  });
  return (
    <Show when={items()} fallback={<p class="text-text-disabled">…</p>}>
      {(held) => (
        <Show when={held().length > 0} fallback={<p class="text-text-faint">{say("run_no_evidence")}</p>}>
          <ul class="text-note">
            <For each={held()}>
              {(item) => (
                <li class="flex items-center gap-base border-b border-g1 py-snug">
                  <span class="w-figure shrink-0 text-text-faint">{say(`evidence_${item.kind}`)}</span>
                  <span class="flex-1 truncate font-mono text-text-quiet">{item.locator}</span>
                  <Show when={item.picture}>
                    {(picture) => (
                      <span class="text-text-disabled">
                        {picture().width}×{picture().height} {picture().media_type}
                      </span>
                    )}
                  </Show>
                  <span class="text-text-disabled">#{item.at}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      )}
    </Show>
  );
}

export function Run(props: RunProps) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const hearing = useHearing();
  const [lens, setLens] = createSignal<Lens>("turns");
  const belief = () => ui.conn.belief.runs[props.run];
  const rounds = createMemo(() => ui.conn.asking.ask({ rounds: { run: props.run } }));
  const answer = createMemo<RoundsAnswer | undefined>(() => {
    const held = rounds()();
    return held !== undefined && "rounds" in held ? held.rounds : undefined;
  });
  const lastFence = createMemo<GitOid | null>(() => {
    const turns = answer()?.turns ?? [];
    for (let i = turns.length - 1; i >= 0; i -= 1) {
      for (const note of turns[i]?.notes ?? []) {
        if ("fenced" in note) return note.fenced.oid;
      }
    }
    return null;
  });
  const live = () => belief()?.doing.kind !== "frozen" && belief() !== undefined;
  const room = () => belief()?.addr ?? null;
  const roomWord = () => {
    const at = room();
    return at === null ? say("talk_resident") : roomOf(at);
  };

  return (
    <div class="mx-auto flex min-h-0 w-full max-w-page flex-1 flex-col px-pane pt-wide">
      <div class="flex flex-wrap items-center gap-base pb-base">
        <Show when={room()}>
          {(at) => (
            <>
              <a href={toFragment({ kind: "building", address: buildingOf(at()) })} class="text-note text-text-faint">
                {buildingOf(at())}
              </a>
              <span class="text-text-disabled">/</span>
              <a href={toFragment({ kind: "talk", address: at() })} class="text-note text-text-faint">
                {roomOf(at())}
              </a>
              <span class="text-text-disabled">/</span>
            </>
          )}
        </Show>
        <h1 class="truncate text-heading font-heading">{answer()?.opening?.task ?? belief()?.task ?? props.run}</h1>
        <span class="flex-1" />
        <Show when={live()}>
          <button
            type="button"
            class="rounded-control px-base py-tight text-label text-text-quiet hover:bg-g1 hover:text-alert"
            onClick={() => command(cancel(props.run))}
          >
            {say("run_cancel")}
          </button>
        </Show>
      </div>
      <Show when={answer()?.opening?.goal}>
        {(goal) => <p class="mb-base text-note text-text-faint">{say("run_goal")}: {goal()}</p>}
      </Show>
      <nav class="mb-base flex gap-tight border-b border-g1 text-label" aria-label={say("run_lenses")}>
        <For each={LENSES}>
          {(each) => (
            <button
              type="button"
              class={`-mb-px border-b-2 px-base py-snug ${lens() === each ? "border-accent text-text" : "border-transparent text-text-faint hover:text-text-quiet"}`}
              aria-current={lens() === each ? "page" : undefined}
              onClick={() => setLens(each)}
            >
              {say(`run_${each}`)}
            </button>
          )}
        </For>
      </nav>
      <div class="min-h-0 flex-1 overflow-y-auto pb-wide">
        <Switch>
          <Match when={lens() === "turns"}>
            <div class="mx-auto max-w-talk">
              <Show when={belief()} fallback={<p class="text-text-disabled">…</p>}>
                {(run) => <Thread run={run()} who={roomWord()} />}
              </Show>
              <Show when={live()}>
                <div class="mt-wide">
                  <Composer
                    placeholder={say("talk_placeholder_room", { room: roomWord() })}
                    sending={sendingInto(belief()?.doing)}
                    draft={props.run}
                    hearing={hearing()}
                    onSend={(text) => command(steer(props.run, text))}
                    onStop={() => command(cancel(props.run))}
                  />
                </div>
              </Show>
            </div>
          </Match>
          <Match when={lens() === "context"}>
            <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
              {(held) => <Context rounds={held()} />}
            </Show>
          </Match>
          <Match when={lens() === "changes"}>
            <Show when={answer()?.opened_at} fallback={<p class="text-text-faint">{say("run_no_fence")}</p>}>
              {(base) => <Changes base={base()} head={lastFence() === base() ? null : lastFence()} />}
            </Show>
          </Match>
          <Match when={lens() === "evidence"}>
            <Evidence run={props.run} />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
