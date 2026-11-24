// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Match, Show, Switch, createEffect, createMemo, createSignal, untrack } from "solid-js";
import { adopted } from "../core/belief/adopted";
import { cancel, steer } from "../core/commands";
import { sendingInto } from "../core/doing";
import { fill, say, type Key } from "../core/lang";
import { NO_TAIL } from "../core/live_output";
import { RUN_LENSES, roomOf, type RunLens } from "../core/route";
import { STOP } from "../core/slash";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { RunId } from "../wire";
import Changes from "./changes.solid";
import { lastCheckpointIn } from "./checkpoints";
import RunMonitor from "./monitor/run_monitor";
import Button from "./parts/button.solid";
import EmptyState from "./parts/empty.solid";
import Tabs from "./parts/tabs.solid";
import RunContext from "./run/context.solid";
import Evidence from "./run/evidence.solid";
import Head from "./run/head.solid";
import type { Share } from "./run/lanes";
import Prompt from "./run/prompt.solid";
import River from "./run/river.solid";
import Composer from "./talk/composer_view";
import Thread from "./talk/thread";

const WORDS: Record<RunLens, Key> = {
  time: "run_time",
  turns: "run_turns",
  monitor: "run_monitor",
  prompt: "run_prompt",
  context: "run_context",
  changes: "run_changes",
  evidence: "run_evidence",
};

export default function Run(props: { readonly run: RunId; readonly lens?: RunLens | undefined }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const tails = storeValue(u.conn.live);
  const [chosen, setChosen] = createSignal<RunLens | null>(untrack(() => props.lens ?? null));
  createEffect(
    () => props.lens,
    (lens) => {
      setChosen(lens ?? null);
    },
  );
  const lenses = createMemo(() => RUN_LENSES.map((id) => ({ id, label: say(lang(), WORDS[id]) })));
  const roundsStore = storeValue(() => u.conn.asking.ask({ rounds: { run: props.run } }));
  const summaryStore = storeValue(() => u.conn.asking.ask({ run_view: { run: props.run } }));
  const rounds = createMemo(() => {
    const value = roundsStore();
    return value !== undefined && "rounds" in value ? value.rounds : undefined;
  });
  const summary = createMemo(() => {
    const value = summaryStore();
    return value !== undefined && "run" in value ? value.run : undefined;
  });
  const mine = createMemo(() => belief().runs[props.run]);
  const shown = createMemo(() => {
    const value = summary();
    return value == null ? mine() : adopted(value, mine());
  });
  const turns = createMemo(() => rounds()?.turns ?? []);
  const checkpoint = createMemo(() => rounds()?.opened_at ?? null);
  const lastCheckpoint = createMemo(() => lastCheckpointIn(turns()));
  const live = createMemo(() => {
    const run = shown();
    return run !== undefined && run.doing.kind !== "frozen";
  });
  const room = createMemo(() => shown()?.addr ?? null);
  const changedTo = createMemo(() =>
    live()
      ? lastCheckpoint() === checkpoint()
        ? null
        : lastCheckpoint()
      : (lastCheckpoint() ?? checkpoint()),
  );
  const firstLens = createMemo<RunLens>(() =>
    live()
      ? "time"
      : lastCheckpoint() !== null && lastCheckpoint() !== checkpoint()
        ? "changes"
        : "turns",
  );
  const current = createMemo(() => chosen() ?? firstLens());
  const from = createMemo(() => rounds()?.opening?.at ?? turns()[0]?.t ?? shown()?.started ?? null);
  const to = createMemo(() => {
    const closing = rounds()?.closing;
    if (closing != null) return closing.at;
    const last = turns().at(-1)?.t ?? from();
    return last === null ? null : live() ? Math.max(last, u.now()) : last;
  });
  const range = createMemo(() => {
    const start = from();
    const end = to();
    return start === null || end === null || turns().length === 0 ? undefined : { start, end };
  });
  const tail = createMemo<Share | null>(() => {
    switch (shown()?.doing.kind) {
      case "calling":
      case "awaiting_reply":
        return "tool";
      case "waiting":
        return "person";
      case "thinking":
        return "model";
      case "frozen":
      case "unknown":
      case undefined:
        return null;
    }
  });
  const roomWord = () => {
    const at = room();
    return at === null ? say(lang(), "talk_resident") : roomOf(at);
  };
  const stop = () => u.send(cancel(props.run));
  const send = (words: string) => u.send(steer(props.run, words));
  function draftSteer(words: string): void {
    u.prefs.setDraft(props.run, words);
    setChosen("turns");
  }
  function pick(id: string): void {
    const value = RUN_LENSES.find((lens) => lens === id);
    if (value !== undefined) setChosen(value);
  }

  return (
    <div class="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-base px-wide py-wide">
      <div class="flex min-w-0 items-start justify-between gap-wide">
        <div class="flex min-w-0 flex-col gap-tight">
          <h1 tabindex={-1} class="truncate text-title font-title">
            {rounds()?.opening?.task ?? shown()?.task ?? props.run}
          </h1>
          <Show when={rounds()?.opening?.goal}>
            {(goal) => (
              <p class="text-note text-text-quiet">
                <span class="text-text-faint">{say(lang(), "run_goal")}</span> {goal()}
              </p>
            )}
          </Show>
          <p class="figure truncate text-note text-text-faint">{props.run}</p>
        </div>
        <Show when={live()}>
          <Button label={STOP} tone="secondary" onPress={stop} />
        </Show>
      </div>
      <Head
        turns={turns()}
        doing={shown()?.doing}
        closing={rounds()?.closing ?? null}
        dispatchedBy={rounds()?.opening?.dispatched_by ?? null}
        room={room()}
        from={from()}
        to={to()}
      />
      <Tabs
        label={say(lang(), "run_lenses")}
        lenses={lenses()}
        current={current()}
        onPick={pick}
        panel={(lens) => (
          <Switch>
            <Match when={lens.id === "time"}>
              <Show when={range()} fallback={<EmptyState missing="run_no_turns" />}>
                {(at) => <River turns={turns()} from={at().start} to={at().end} tail={tail()} />}
              </Show>
            </Match>
            <Match when={lens.id === "turns"}>
              <div class="mx-auto max-w-talk">
                <Show when={shown()} fallback={<p class="text-text-faint">…</p>}>
                  {(run) => <Thread run={run()} stream={mine()} />}
                </Show>
                <Show when={live()}>
                  <div class="mt-wide">
                    <Composer
                      placeholder={fill(say(lang(), "talk_placeholder_room"), { room: roomWord() })}
                      sending={sendingInto(shown()?.doing)}
                      draft={props.run}
                      hearing={u.hearing()}
                      onSend={send}
                      onStop={stop}
                    />
                  </div>
                </Show>
              </div>
            </Match>
            <Match when={lens.id === "monitor"}>
              <div class="flex h-[70vh] min-h-0 flex-col rounded-card border border-edge">
                <RunMonitor
                  turns={turns()}
                  tail={tails()[props.run] ?? NO_TAIL}
                  live={live()}
                  onDraft={draftSteer}
                  onSteer={send}
                />
              </div>
            </Match>
            <Match when={lens.id === "prompt"}>
              <Prompt run={props.run} />
            </Match>
            <Match when={lens.id === "context"}>
              <RunContext turns={turns()} />
            </Match>
            <Match when={lens.id === "changes"}>
              <Show when={checkpoint()} fallback={<EmptyState missing="run_no_checkpoint" />}>
                {(base) => <Changes base={base()} head={changedTo()} />}
              </Show>
            </Match>
            <Match when={lens.id === "evidence"}>
              <Evidence run={props.run} />
            </Match>
          </Switch>
        )}
      />
    </div>
  );
}
