// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One building, one click below the city: one information bar, the
// directory tree, whatever the tree picked, and - where the screen
// affords a third column - the rooms of this building with whoever is
// working in them.
//
// The tree is a column from 1024 px up and a panel a button opens below
// that, so a narrow screen keeps one column without losing the way in.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { halt, pursue, release } from "../core/commands";
import { roomOf, toFragment } from "../core/route";
import type { Address, BuildingAnswer } from "../wire";
import { Address as AddressSchema } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { Commits } from "./building/commits";
import { Directory } from "./building/directory";
import { FileView } from "./building/file";
import { Plan } from "./building/plan";
import { Tree } from "./building/tree";
import type { Picked } from "./building/tree";
import { Badge } from "./parts/badge";
import { Button } from "./parts/button";

export interface BuildingProps {
  readonly address: Address;
}

// What the right-hand pane shows: the plan, the commits, or what the
// tree picked.
type Shown = { readonly kind: "plan" } | { readonly kind: "commits" } | Picked;

const PLAN: Shown = { kind: "plan" };
const COMMITS: Shown = { kind: "commits" };

function Rooms(props: { readonly answer: BuildingAnswer; readonly onPick: (picked: Picked) => void }) {
  const ui = useUi();
  const say = useSay();
  const livingIn = (room: Address) =>
    Object.values(ui.conn.belief.runs).filter(
      (run) => run.doing.kind !== "frozen" && run.addr !== null && (run.addr === room || run.addr.startsWith(`${room}/`)),
    ).length;
  return (
    <div>
      <h2 class="mb-base text-label font-label text-text-quiet">{say("bld_rooms")}</h2>
      <Show when={props.answer.rooms.length > 0} fallback={<p class="text-note text-text-faint">{say("bld_no_rooms")}</p>}>
        <ul class="text-note">
          <For each={props.answer.rooms}>
            {(name) => {
              const at = () => AddressSchema.make(`${props.answer.addr}/${name}`);
              return (
                <li>
                  <button
                    type="button"
                    class="flex h-step w-full items-center gap-snug rounded-control px-snug text-left leading-none text-text-quiet hover:bg-g1"
                    onClick={() => {
                      props.onPick({ at: at(), kind: "directory" });
                    }}
                  >
                    <span class="min-w-0 flex-1 truncate">{roomOf(at())}</span>
                    <Show when={livingIn(at())}>
                      {(n) => <Badge text={say("city_active", { n: String(n()) })} weight="live" dot />}
                    </Show>
                  </button>
                </li>
              );
            }}
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
  const [shown, setShown] = createSignal<Shown>(PLAN);
  const [treeOpen, setTreeOpen] = createSignal(false);
  const picked = (): Picked | null => {
    const held = shown();
    return held.kind === "file" || held.kind === "directory" ? held : null;
  };
  const pick = (next: Shown) => {
    setShown(next);
    setTreeOpen(false);
  };
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
    if (held === undefined || !("planned" in held.progress) || held.progress.planned.total === 0) return null;
    return held.progress.planned;
  });
  const setGoalNow = () => {
    const words = goal().trim();
    if (words !== "" && command(pursue(props.address, { set: { goal: words } }))) setGoal("");
  };

  return (
    <div class="flex min-h-0 w-full flex-1 flex-col">
      <header class="flex flex-wrap items-center gap-base border-b border-g2 px-pane py-snug" aria-label={say("bld_bar")}>
        <a href={toFragment({ kind: "city" })} class="text-note text-text-faint hover:text-text-quiet">
          {say("nav_city")}
        </a>
        <span class="text-text-disabled" aria-hidden="true">
          /
        </span>
        <h1 class="text-title font-title">{props.address}</h1>
        <Show when={done()}>
          {(p) => <Badge text={say("bld_progress", { done: String(p().done), total: String(p().total) })} />}
        </Show>
        <span class="text-text-disabled" aria-hidden="true">
          ⚑
        </span>
        <Show
          when={pursuit()}
          fallback={
            <>
              <input
                class="min-w-0 flex-1 bg-transparent py-tight text-note outline-none placeholder:text-text-disabled"
                placeholder={say("bld_goal_placeholder")}
                value={goal()}
                onInput={(event) => setGoal(event.currentTarget.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") setGoalNow();
                }}
              />
              <Show when={goal().trim() !== ""}>
                <Button label={say("bld_pursue")} tone="primary" onPress={setGoalNow} />
              </Show>
            </>
          }
        >
          {(line) => (
            <>
              <span class={`inline-block size-dot rounded-pill ${line().state === "running" ? "bg-accent" : "bg-g4"}`} />
              <span class="min-w-0 flex-1 truncate text-note text-text-quiet">{line().goal}</span>
              <span class="font-mono text-note text-text-disabled">{line().verdict}</span>
              <Button
                label={line().state === "running" ? say("bld_pause") : say("bld_resume")}
                tone="quiet"
                onPress={() => command(pursue(props.address, line().state === "running" ? "pause" : "resume"))}
              />
              <Button label={say("bld_clear")} tone="quiet" onPress={() => command(pursue(props.address, "clear"))} />
            </>
          )}
        </Show>
        <Button
          label={halted() ? say("bld_release", { addr: props.address }) : say("bld_halt", { addr: props.address })}
          tone={halted() ? "secondary" : "quiet"}
          onPress={() => command(halted() ? release({ building: props.address }) : halt({ building: props.address }))}
        />
        <span class="lg:hidden">
          <Button label={say("bld_tree")} tone="quiet" onPress={() => setTreeOpen((held) => !held)} />
        </span>
      </header>

      <div class="flex min-h-0 flex-1 flex-col lg:flex-row">
        <aside
          class={`shrink-0 overflow-y-auto border-g2 px-snug py-base lg:block lg:w-tree lg:border-r ${treeOpen() ? "block border-b" : "hidden"}`}
        >
          <button
            type="button"
            class={`mb-tight flex h-step w-full items-center rounded-control pl-tight pr-snug text-left text-note leading-none ${shown().kind === "plan" ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"}`}
            aria-current={shown().kind === "plan" ? "true" : undefined}
            onClick={() => {
              pick(PLAN);
            }}
          >
            <span class="flex w-base shrink-0 justify-center text-text-disabled">≡</span>
            <span class="ml-tight">{say("bld_plan")}</span>
          </button>
          <button
            type="button"
            class={`mb-tight flex h-step w-full items-center rounded-control pl-tight pr-snug text-left text-note leading-none ${shown().kind === "commits" ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"}`}
            aria-current={shown().kind === "commits" ? "true" : undefined}
            onClick={() => {
              pick(COMMITS);
            }}
          >
            <span class="flex w-base shrink-0 justify-center font-mono text-text-disabled">⎇</span>
            <span class="ml-tight">{say("bld_commits")}</span>
          </button>
          <Tree root={props.address} picked={picked()} onPick={pick} />
        </aside>
        <section class="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto px-pane py-base">
          <Switch>
            <Match when={shown().kind === "plan"}>
              <Show when={building()} fallback={<p class="text-text-disabled">…</p>}>
                {(held) => <Plan answer={held()} />}
              </Show>
            </Match>
            <Match when={shown().kind === "commits"}>
              <Commits building={props.address} />
            </Match>
            <Match when={picked()?.kind === "file" ? picked() : undefined}>
              {(file) => <FileView at={file().at} root={props.address} />}
            </Match>
            <Match when={picked()?.kind === "directory" ? picked() : undefined}>
              {(dir) => <Directory at={dir().at} root={props.address} onPick={pick} />}
            </Match>
          </Switch>
        </section>
        <aside class="hidden shrink-0 overflow-y-auto border-l border-g2 px-pane py-base wide:block wide:w-tree" aria-label={say("bld_rooms")}>
          <Show when={building()}>{(held) => <Rooms answer={held()} onPick={pick} />}</Show>
        </aside>
      </div>
    </div>
  );
}
