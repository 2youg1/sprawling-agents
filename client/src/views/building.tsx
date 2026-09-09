// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One building, one click below the city: the directory tree on the
// left - the rules, the plan, the rooms and what each room wrote - and
// on the right whatever was picked: the plan as rows, a file as it is
// on disk, or a room as the runs that worked in it. The building's
// standing goal and its own brake sit in the head.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { halt, pursue, release } from "../core/commands";
import { roomOf, toFragment } from "../core/route";
import { clock, kib } from "../core/time";
import type { Address, BuildingAnswer, PlanRow } from "../wire";
import { useCommand, useLang, useSay, useUi } from "../ui";
import { Tree } from "./building/tree";
import type { Picked } from "./building/tree";
import { Prose } from "./prose";

export interface BuildingProps {
  readonly address: Address;
}

function statusWord(say: ReturnType<typeof useSay>, row: PlanRow): string {
  if (row.status === "not_started" && row.ready) return say("status_ready");
  return say(`status_${row.status}`);
}

function Plan(props: { readonly answer: BuildingAnswer }) {
  const say = useSay();
  return (
    <div>
      <Show when={props.answer.problems.length > 0}>
        <ul class="mb-base rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
          <For each={props.answer.problems}>{(problem) => <li>{problem}</li>}</For>
        </ul>
      </Show>
      <Show when={props.answer.plan.length > 0} fallback={<p class="text-text-faint">{say("plan_empty")}</p>}>
        <table class="w-full border-collapse text-note">
          <tbody>
            <For each={props.answer.plan}>
              {(row) => {
                const depth = () => row.node.split(".").length - 1;
                return (
                  <tr class="border-b border-g1">
                    <td class="w-figure py-snug pr-snug font-mono text-text-faint" style={{ "padding-left": `${String(depth() * 16)}px` }}>
                      {row.node}
                    </td>
                    <td class={`py-snug pr-snug ${row.status === "done" ? "text-text-faint" : "text-text"}`}>
                      {row.item}
                      <Show when={row.needs.length > 0}>
                        <span class="ml-snug text-text-disabled">← {row.needs.join(", ")}</span>
                      </Show>
                    </td>
                    <td class="py-snug text-right whitespace-nowrap">
                      <span
                        class={`rounded-pill px-snug py-tight ${
                          row.status === "done"
                            ? "bg-g2 text-text-faint"
                            : row.status === "blocked"
                              ? "bg-alert text-g0"
                              : row.status === "in_progress"
                                ? "bg-accent text-g0"
                                : row.ready
                                  ? "bg-g3 text-text"
                                  : "text-text-disabled"
                        }`}
                      >
                        {statusWord(say, row)}
                      </span>
                    </td>
                  </tr>
                );
              }}
            </For>
          </tbody>
        </table>
      </Show>
      <Show when={props.answer.blocked.length > 0}>
        <ul class="mt-base text-note text-text-quiet">
          <For each={props.answer.blocked}>
            {(line) => (
              <li class="my-tight">
                <span class="text-alert">{line.source}</span> · {line.line}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function FileView(props: { readonly at: Address }) {
  const ui = useUi();
  const say = useSay();
  const document = createMemo(() => ui.conn.asking.ask({ document: { at: props.at } }));
  const doc = createMemo(() => {
    const answer = document()();
    if (answer === undefined) return undefined;
    return "document" in answer ? answer.document : null;
  });
  const markdown = () => props.at.endsWith(".md");
  return (
    <Show when={doc()} fallback={<p class="text-text-disabled">{doc() === null ? say("file_missing") : "…"}</p>}>
      {(held) => (
        <div>
          <Show when={held().binary}>
            <p class="text-text-faint">{say("file_binary", { kib: kib(held().bytes) })}</p>
          </Show>
          <Show when={held().truncated}>
            <p class="mb-base text-note text-alert">
              {say("file_truncated", { kib: kib(held().text.length), total: kib(held().bytes) })}
            </p>
          </Show>
          <Show when={!held().binary}>
            <Show when={markdown()} fallback={<pre class="overflow-x-auto font-mono text-note leading-relaxed text-text-quiet">{held().text}</pre>}>
              <Prose text={held().text} />
            </Show>
          </Show>
        </div>
      )}
    </Show>
  );
}

function RoomView(props: { readonly at: Address }) {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const runs = createMemo(() =>
    Object.values(ui.conn.belief.runs)
      .filter((run) => run.addr === props.at)
      .sort((a, b) => (b.started ?? 0) - (a.started ?? 0)),
  );
  return (
    <div>
      <div class="mb-base flex items-center justify-between">
        <h2 class="text-heading font-heading">{roomOf(props.at)}</h2>
        <a href={toFragment({ kind: "talk", address: props.at })} class="rounded-control bg-g2 px-base py-tight text-label hover:bg-g3">
          {say("bld_open_talk")}
        </a>
      </div>
      <Show when={runs().length > 0} fallback={<p class="text-text-faint">{say("tree_no_runs")}</p>}>
        <ul>
          <For each={runs()}>
            {(run) => (
              <li class="border-b border-g1 py-snug text-note">
                <a href={toFragment({ kind: "run", run: run.run })} class="flex items-center gap-base hover:text-text">
                  <span class={`inline-block size-dot rounded-pill ${run.doing.kind === "frozen" ? "bg-g4" : "bg-accent"}`} />
                  <span class="flex-1 truncate text-text-quiet">{run.task ?? run.run}</span>
                  <Show when={run.started}>{(at) => <span class="text-text-disabled">{clock(lang(), at())}</span>}</Show>
                </a>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

export function Building(props: BuildingProps) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [picked, setPicked] = createSignal<Picked | null>(null);
  const [goal, setGoal] = createSignal("");
  const answer = createMemo(() => ui.conn.asking.ask({ building_view: { addr: props.address } }));
  const building = createMemo(() => {
    const held = answer()();
    return held !== undefined && "building" in held ? held.building : undefined;
  });
  const city = ui.conn.asking.ask("city_view");
  const pursuit = createMemo(() => {
    const held = city();
    return held !== undefined && "city" in held ? held.city.pursuits.find((line) => line.addr === props.address) : undefined;
  });
  const halted = () => ui.conn.belief.halted.includes(props.address);
  const done = createMemo(() => {
    const held = building();
    if (held === undefined || !("planned" in held.progress)) return null;
    return held.progress.planned;
  });

  return (
    <div class="mx-auto flex min-h-0 w-full max-w-page flex-1 flex-col px-pane pt-wide">
      <div class="flex flex-wrap items-center gap-base pb-base">
        <a href={toFragment({ kind: "city" })} class="text-note text-text-faint">
          {say("nav_city")}
        </a>
        <span class="text-text-disabled">/</span>
        <h1 class="text-title font-title">{props.address}</h1>
        <Show when={done()}>
          {(p) => (
            <span class="text-note text-text-faint">
              {say("bld_progress", { done: String(p().done), total: String(p().total) })}
            </span>
          )}
        </Show>
        <span class="flex-1" />
        <button
          type="button"
          class={`rounded-control px-base py-tight text-label hover:bg-g1 ${halted() ? "text-accent" : "text-text-quiet hover:text-alert"}`}
          onClick={() => command(halted() ? release({ building: props.address }) : halt({ building: props.address }))}
        >
          {halted() ? say("bld_release") : say("bld_halt")}
        </button>
      </div>

      <div class="mb-base flex flex-wrap items-center gap-snug rounded-card bg-g1 px-base py-snug text-note">
        <Show
          when={pursuit()}
          fallback={
            <>
              <input
                class="min-w-measure flex-1 rounded-control bg-g2 px-base py-tight text-note outline-none placeholder:text-text-disabled"
                placeholder={say("bld_goal_placeholder")}
                value={goal()}
                onInput={(event) => setGoal(event.currentTarget.value)}
              />
              <button
                type="button"
                class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
                disabled={goal().trim() === ""}
                onClick={() => {
                  if (command(pursue(props.address, { set: { goal: goal().trim() } }))) setGoal("");
                }}
              >
                {say("bld_pursue")}
              </button>
            </>
          }
        >
          {(line) => (
            <>
              <span class={line().state === "running" ? "text-accent" : "text-text-faint"}>
                {line().state === "running" ? say("bld_pursuing") : say("bld_paused")}
              </span>
              <span class="flex-1 truncate text-text-quiet">{line().goal}</span>
              <span class="text-text-disabled">{line().verdict}</span>
              <button
                type="button"
                class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
                onClick={() => command(pursue(props.address, line().state === "running" ? "pause" : "resume"))}
              >
                {line().state === "running" ? say("bld_pause") : say("bld_resume")}
              </button>
              <button
                type="button"
                class="rounded-control px-snug py-tight text-label text-text-faint hover:bg-g2 hover:text-alert"
                onClick={() => command(pursue(props.address, "clear"))}
              >
                {say("bld_clear")}
              </button>
            </>
          )}
        </Show>
      </div>

      <div class="flex min-h-0 flex-1 gap-wide pb-wide">
        <aside class="w-tree shrink-0 overflow-y-auto rounded-panel bg-g1/60 p-snug">
          <button
            type="button"
            class={`mb-tight w-full rounded-control px-snug py-tight text-left text-note ${picked() === null ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"}`}
            onClick={() => setPicked(null)}
          >
            {say("bld_plan")}
          </button>
          <Tree root={props.address} picked={picked()} onPick={setPicked} />
        </aside>
        <section class="min-w-0 flex-1 overflow-y-auto">
          <Switch>
            <Match when={picked() === null}>
              <Show when={building()} fallback={<p class="text-text-disabled">…</p>}>
                {(held) => <Plan answer={held()} />}
              </Show>
            </Match>
            <Match when={picked()?.kind === "file" ? picked() : undefined}>
              {(file) => (
                <div>
                  <p class="mb-base font-mono text-note text-text-faint">{file().at}</p>
                  <FileView at={file().at} />
                </div>
              )}
            </Match>
            <Match when={picked()?.kind === "directory" ? picked() : undefined}>{(dir) => <RoomView at={dir().at} />}</Match>
          </Switch>
        </section>
      </div>
    </div>
  );
}
