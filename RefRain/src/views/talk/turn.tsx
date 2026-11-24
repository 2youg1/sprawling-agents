// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import type { Doing } from "../../core/doing";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { count } from "../../core/time";
import { ui } from "../../ui";
import type { RunId, Turn } from "../../wire";
import type { Phase } from "../runs/lineage";
import { arrivalsOf, filedArrivals } from "./arrivals";
import { Calls } from "./call_line_view";
import ForkButton from "./fork_button_view";
import type { ForkEntry, ForkPlan } from "./forking";
import Grounded from "./grounded";
import Head from "./head_view";
import { noteAt } from "./note_line";
import NoteLine from "./note_line_view";
import Reasoning from "./reasoning_view";
import { rhythmOf } from "./rhythm";
import { cutOff, silentTurn, type Phase as Live } from "./silence";
import type { DocumentSources } from "./source_context";
import { tpsOf, ttftTookOf } from "./timing";

export interface TurnProps {
  readonly turn: Turn;
  readonly sources?: DocumentSources | undefined;
  readonly run: RunId;
  readonly who: string;
  readonly model: string | null;
  readonly live: Live;
  readonly doing: Doing | undefined;
  readonly showEmpty: boolean;
  readonly ceiling: number | null;
  readonly whole: boolean;
  readonly onFork?: ((plan: ForkPlan) => void) | undefined;
  readonly onHover: (entry: ForkEntry | null) => void;
}

const EMPTY: DocumentSources = new Map();

export default function TurnView(props: TurnProps) {
  const lang = storeValue(ui().lang);
  const filed = storeValue(filedArrivals);
  const said = createMemo(() => props.turn.said ?? "");
  const silence = createMemo(() => silentTurn(props.turn, props.live));
  const cutoff = createMemo(() => cutOff(props.turn.stopped));
  const phase = createMemo<Phase>(() =>
    props.turn.notes.some((note) => "waiting" in note)
      ? "person"
      : props.turn.calls.length > 0
        ? "tool"
        : "model",
  );
  const rhythm = createMemo(() => {
    const arrivals = arrivalsOf(props.run, props.turn.opened, filed());
    return arrivals === null ? null : rhythmOf(arrivals);
  });
  const arrivals = createMemo(() => props.turn.notes.filter((notice) => "arrived" in notice));
  const other = createMemo(() => props.turn.notes.filter((notice) => !("arrived" in notice)));
  const thought = createMemo(() => (props.whole ? props.turn.thought : null));
  const showsHead = createMemo(() => said() !== "" || props.model !== null);
  const silenceWord = createMemo(() =>
    props.ceiling === null
      ? say(lang(), "talk_said_nothing")
      : fill(say(lang(), "talk_said_nothing_capped"), { n: count(props.ceiling) }),
  );

  return (
    <div class="flex flex-col gap-snug pb-section last:pb-0" data-wear={phase()}>
      <For each={arrivals()} keyed={noteAt}>
        {(notice) => (
          <NoteLine
            note={notice()}
            turn={props.turn}
            run={props.run}
            onFork={props.onFork}
            onHover={props.onHover}
          />
        )}
      </For>
      <Show when={showsHead() || Boolean(thought())}>
        <div class="group flex flex-col gap-snug">
          <Show when={showsHead()}>
            <div class="flex items-start justify-between gap-snug">
              <Head
                who={props.who}
                at={props.turn.t}
                model={props.model}
                ttft={said() === "" ? null : ttftTookOf(props.turn)}
                tps={said() === "" ? null : tpsOf(props.turn)}
                rhythm={rhythm()}
              />
              <Show when={props.onFork}>
                {(fork) => (
                  <ForkButton
                    entry={{ kind: "turn", turn: props.turn }}
                    run={props.run}
                    onFork={fork()}
                    onHover={props.onHover}
                  />
                )}
              </Show>
            </div>
          </Show>
          <Show when={thought()}>{(text) => <Reasoning text={text()} open={false} />}</Show>
          <Show when={said() !== ""}>
            <div class="text-body">
              <Grounded text={said()} sources={props.sources ?? EMPTY} />
            </div>
          </Show>
        </div>
      </Show>
      <Show when={props.whole && props.turn.calls.length > 0}>
        <Calls
          calls={props.turn.calls}
          run={props.run}
          turn={props.turn}
          doing={props.doing}
          onFork={props.onFork}
          onHover={props.onHover}
        />
      </Show>
      <Show when={silence() && props.showEmpty}>
        <div class="rounded-card border border-alert/40 px-base py-snug text-note text-alert">
          {silenceWord()}
        </div>
      </Show>
      <Show when={cutoff()}>
        {(cut) => (
          <div class="text-note text-alert">
            {fill(say(lang(), "talk_cut_off"), { why: cut() })}
          </div>
        )}
      </Show>
      <For each={other()} keyed={noteAt}>
        {(notice) => (
          <NoteLine
            note={notice()}
            turn={props.turn}
            run={props.run}
            onFork={props.onFork}
            onHover={props.onHover}
          />
        )}
      </For>
    </div>
  );
}
