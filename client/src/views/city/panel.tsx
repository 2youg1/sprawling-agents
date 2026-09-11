// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What was picked on the drawing, in words: how far the plan has got,
// what is stuck, the standing goal, and the runs that worked there. It
// is the third column on a wide screen, a drawer over the drawing on a
// medium one, and a block under the drawing on a narrow one - one
// component either way, so the three widths cannot drift apart.

import { For, Show, createMemo } from "solid-js";

import { clock } from "../../core/time";
import { toFragment } from "../../core/route";
import type { Address, CityAnswer } from "../../wire";
import { useLang, useSay, useUi } from "../../ui";
import { Badge } from "../parts/badge";

export interface PanelProps {
  readonly addr: Address;
  readonly city: CityAnswer;
  readonly onClose: () => void;
}

export function Panel(props: PanelProps) {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const building = createMemo(() => props.city.buildings.find((each) => each.addr === props.addr));
  const planned = createMemo(() => {
    const held = building();
    if (held === undefined || !("planned" in held.progress)) return null;
    return held.progress.planned;
  });
  const pursuit = createMemo(() => props.city.pursuits.find((line) => line.addr === props.addr));
  const runs = createMemo(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr !== null && (run.addr === props.addr || run.addr.startsWith(`${props.addr}/`)))
      .sort((a, b) => (b.started ?? 0) - (a.started ?? 0)),
  );

  return (
    <div class="flex flex-col gap-base">
      <div class="flex items-baseline gap-snug">
        <h2 class="min-w-0 flex-1 truncate text-heading font-heading">{props.addr}</h2>
        <button
          type="button"
          class="rounded-control px-snug text-label text-text-faint hover:bg-g2"
          onClick={() => {
            props.onClose();
          }}
        >
          {say("panel_close")}
        </button>
      </div>
      <div class="flex flex-wrap items-center gap-snug">
        <Show when={planned()}>
          {(plan) => (
            <>
              <Badge text={say("bld_progress", { done: String(plan().done), total: String(plan().total) })} />
              <Show when={building()?.ready}>
                {(ready) => <Badge text={say("city_ready", { n: String(ready()) })} weight="live" />}
              </Show>
            </>
          )}
        </Show>
        <Show when={building()?.blocked.length}>
          {(stuck) => <Badge text={say("city_stuck", { n: String(stuck()) })} weight="alert" />}
        </Show>
      </div>
      <a
        href={toFragment({ kind: "building", address: props.addr })}
        class="self-start rounded-control bg-g2 px-base py-tight text-label hover:bg-g3"
      >
        {say("city_enter")}
      </a>
      <Show when={pursuit()}>
        {(line) => (
          <p class="flex items-baseline gap-snug text-note">
            <span class={`inline-block size-dot shrink-0 rounded-pill ${line().state === "running" ? "bg-accent" : "bg-g4"}`} />
            <span class="min-w-0 flex-1 text-text-quiet">{line().goal}</span>
          </p>
        )}
      </Show>
      <Show when={(building()?.problems.length ?? 0) > 0}>
        <ul class="rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
          <For each={building()?.problems}>{(problem) => <li>{problem}</li>}</For>
        </ul>
      </Show>
      <Show when={(building()?.blocked.length ?? 0) > 0}>
        <ul class="text-note text-text-quiet">
          <For each={building()?.blocked}>
            {(line) => (
              <li class="my-tight">
                <span class="text-alert">{line.source}</span> <span class="text-text-faint">{line.line}</span>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <section>
        <h3 class="mb-tight text-label font-label text-text-quiet">{say("city_runs")}</h3>
        <Show when={runs().length > 0} fallback={<p class="text-note text-text-faint">{say("tree_no_runs")}</p>}>
          <ul class="text-note">
            <For each={runs().slice(0, 8)}>
              {(run) => (
                <li class="border-b border-g1 py-snug">
                  <a href={toFragment({ kind: "run", run: run.run })} class="flex items-center gap-snug hover:text-text">
                    <span class={`inline-block size-dot shrink-0 rounded-pill ${run.doing.kind === "frozen" ? "bg-g4" : "bg-accent"}`} />
                    <span class="min-w-0 flex-1 truncate text-text-quiet">{run.task ?? run.run}</span>
                    <Show when={run.started}>{(at) => <span class="shrink-0 text-text-disabled">{clock(lang(), at())}</span>}</Show>
                  </a>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </div>
  );
}
