// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city, drawn: one block per building, City Hall at the head of the
// avenue, a lit window per run and a figure at the door for each run
// still going. It folds the detail away on purpose - who is doing what
// is one click down, on the building - and keeps only what a glance
// wants: where work is moving, where it is stuck, where somebody is
// waiting for a person. One control, the brake.

import { For, Show, createMemo } from "solid-js";

import type { RunBelief } from "../core/belief";
import { halt, release } from "../core/commands";
import { toFragment } from "../core/route";
import type { BuildingProgress, PursuitLine } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { squircle } from "./city/shape";

const W = 176;
const H = 116;
const HALL_W = 208;
const HALL_H = 124;
const GAP = 44;
const COLUMNS = 4;

interface Placed {
  readonly building: BuildingProgress;
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
  readonly runs: readonly RunBelief[];
  readonly pursuit: PursuitLine | undefined;
}

function ratio(building: BuildingProgress): { done: number; blocked: number } {
  if ("planned" in building.progress) {
    const p = building.progress.planned;
    return { done: p.done_ppb / 1e9, blocked: p.blocked_ppb / 1e9 };
  }
  return { done: 0, blocked: 0 };
}

function Figure(props: { readonly run: RunBelief; readonly x: number; readonly y: number; readonly delay: number }) {
  const posture = () => props.run.doing.kind;
  return (
    <g
      class="bob"
      style={{ "animation-delay": `${String(props.delay)}ms`, "transform-origin": `${String(props.x)}px ${String(props.y)}px` }}
    >
      <title>{`${props.run.addr ?? ""} · ${posture()}`}</title>
      <path
        d={`M${String(props.x - 6)} ${String(props.y)} q6 -14 12 0 z`}
        class={posture() === "waiting" ? "fill-alert" : "fill-g8"}
      />
      <circle cx={props.x} cy={props.y - 17} r="5" class={posture() === "waiting" ? "fill-alert" : "fill-g9"} />
      <Show when={posture() === "thinking"}>
        <g class="blink">
          <circle cx={props.x + 8} cy={props.y - 27} r="1.4" class="fill-g7" />
          <circle cx={props.x + 12} cy={props.y - 31} r="1.8" class="fill-g7" />
          <circle cx={props.x + 17} cy={props.y - 36} r="2.4" class="fill-g7" />
        </g>
      </Show>
      <Show when={posture() === "calling"}>
        <rect x={props.x + 6} y={props.y - 12} width="7" height="7" rx="1.5" class="fill-accent" />
      </Show>
      <Show when={posture() === "waiting"}>
        <circle cx={props.x} cy={props.y - 17} r="9" class="fill-none stroke-alert" stroke-width="1.5" />
      </Show>
    </g>
  );
}

function Block(props: { readonly placed: Placed; readonly hall: boolean }) {
  const say = useSay();
  const p = () => props.placed;
  const active = createMemo(() => p().runs.filter((run) => run.doing.kind !== "frozen"));
  const windows = createMemo(() => p().runs.slice(-12));
  const bars = createMemo(() => ratio(p().building));
  const stuck = () => p().building.blocked.length;
  const inner = () => ({ x: p().x + 12, y: p().y + 30, w: p().w - 24, h: p().h - 30 - 22 });

  return (
    <a href={toFragment({ kind: "building", address: p().building.addr })} class="group">
      <title>{`${p().building.addr} · ${String(active().length)} ${say("city_at_work")}`}</title>
      <path
        d={squircle({ x: p().x, y: p().y, w: p().w, h: p().h }, props.hall ? 16 : 12, 4)}
        class={`${props.hall ? "fill-g2" : "fill-g1"} stroke-g3 transition-colors group-hover:fill-g3`}
        stroke-width="1"
      />
      <text x={p().x + 14} y={p().y + 19} class="fill-text-quiet text-label font-label" font-size="13">
        {props.hall ? say("city_hall") : p().building.addr}
      </text>
      <Show when={stuck() > 0}>
        <g>
          <rect x={p().x + p().w - 34} y={p().y + 8} width="24" height="14" rx="7" class="fill-alert" />
          <text x={p().x + p().w - 22} y={p().y + 19} text-anchor="middle" font-size="10" class="fill-g0 font-label">
            {stuck()}
          </text>
        </g>
      </Show>
      <Show when={p().pursuit}>
        {(line) => (
          <g>
            <line x1={p().x + p().w - 6} y1={p().y - 14} x2={p().x + p().w - 6} y2={p().y + 2} class="stroke-g6" stroke-width="1" />
            <path
              d={`M${String(p().x + p().w - 6)} ${String(p().y - 14)} l-14 4 l14 4 z`}
              class={line().state === "running" ? "fill-accent" : "fill-g5"}
            />
          </g>
        )}
      </Show>
      <For each={windows()}>
        {(run, index) => {
          const col = () => index() % 6;
          const row = () => Math.floor(index() / 6);
          const lit = () => run.doing.kind !== "frozen";
          return (
            <rect
              x={inner().x + col() * 24}
              y={inner().y + row() * 20}
              width="14"
              height="12"
              rx="2"
              class={lit() ? (run.doing.kind === "calling" ? "fill-accent blink" : "fill-accent") : "fill-g3"}
              opacity={lit() ? 0.95 : 0.7}
            />
          );
        }}
      </For>
      <rect x={p().x + 12} y={p().y + p().h - 12} width={p().w - 24} height="4" rx="2" class="fill-g3" />
      <rect x={p().x + 12} y={p().y + p().h - 12} width={(p().w - 24) * bars().done} height="4" rx="2" class="fill-accent" />
      <rect
        x={p().x + 12 + (p().w - 24) * bars().done}
        y={p().y + p().h - 12}
        width={(p().w - 24) * bars().blocked}
        height="4"
        rx="2"
        class="fill-alert"
      />
      <For each={active().slice(0, 4)}>
        {(run, index) => (
          <Figure run={run} x={p().x + p().w - 24 - index() * 20} y={p().y + p().h + 14} delay={index() * 400} />
        )}
      </For>
    </a>
  );
}

export function City() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const city = ui.conn.asking.ask("city_view");

  const placed = createMemo<Placed[]>(() => {
    const answer = city();
    if (answer === undefined || !("city" in answer)) return [];
    const buildings = [...answer.city.buildings].sort((a, b) =>
      a.addr === "hall" ? -1 : b.addr === "hall" ? 1 : a.addr.localeCompare(b.addr),
    );
    const runs = Object.values(ui.conn.belief.runs);
    const pursuits = answer.city.pursuits;
    const others = buildings.filter((b) => b.addr !== "hall");
    const columns = Math.max(1, Math.min(COLUMNS, others.length));
    const width = columns * W + (columns - 1) * GAP;
    const out: Placed[] = [];
    const hall = buildings.find((b) => b.addr === "hall");
    const top = hall === undefined ? 0 : HALL_H + GAP + 24;
    if (hall !== undefined) {
      out.push({
        building: hall,
        x: (Math.max(width, HALL_W) - HALL_W) / 2,
        y: 16,
        w: HALL_W,
        h: HALL_H,
        runs: runs.filter((run) => run.addr !== null && (run.addr === "hall" || run.addr.startsWith("hall/"))),
        pursuit: pursuits.find((line) => line.addr === "hall"),
      });
    }
    others.forEach((building, index) => {
      const col = index % columns;
      const row = Math.floor(index / columns);
      out.push({
        building,
        x: col * (W + GAP) + (Math.max(width, HALL_W) - width) / 2,
        y: 16 + top + row * (H + GAP + 24),
        w: W,
        h: H,
        runs: runs.filter(
          (run) => run.addr !== null && (run.addr === building.addr || run.addr.startsWith(`${building.addr}/`)),
        ),
        pursuit: pursuits.find((line) => line.addr === building.addr),
      });
    });
    return out;
  });
  const bounds = createMemo(() => {
    const all = placed();
    const w = Math.max(HALL_W, ...all.map((p) => p.x + p.w)) + 16;
    const h = Math.max(HALL_H, ...all.map((p) => p.y + p.h)) + 48;
    return { w, h };
  });
  const halted = () => ui.conn.belief.halted.includes("city");
  const active = createMemo(() => Object.values(ui.conn.belief.runs).filter((run) => run.doing.kind !== "frozen").length);

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="mx-auto flex w-full max-w-page items-baseline justify-between px-pane pt-wide">
        <h1 class="text-title font-title">{ui.conn.belief.city ?? say("nav_city")}</h1>
        <p class="text-note text-text-faint">
          {active() > 0 ? say("city_active", { n: String(active()) }) : say("city_quiet")}
        </p>
      </div>
      <div class="min-h-0 flex-1 overflow-auto px-pane py-wide">
        <Show
          when={placed().length > 0}
          fallback={<p class="mx-auto max-w-measure text-center text-text-faint">{say("city_no_buildings")}</p>}
        >
          <svg
            viewBox={`0 0 ${String(bounds().w)} ${String(bounds().h)}`}
            class={`mx-auto block max-w-page transition-opacity ${halted() ? "opacity-40" : ""}`}
            style={{ width: `${String(bounds().w)}px`, "max-width": "100%" }}
            role="img"
            aria-label={say("city_drawing")}
          >
            <For each={placed()}>{(p) => <Block placed={p} hall={p.building.addr === "hall"} />}</For>
          </svg>
        </Show>
      </div>
      <div class="mx-auto flex w-full max-w-page items-center justify-between px-pane pb-pane text-note text-text-faint">
        <Show when={halted()} fallback={<span>{say("city_legend")}</span>}>
          <span class="text-alert">{say("city_stopped_line")}</span>
        </Show>
        <button
          type="button"
          class={`rounded-control px-base py-tight text-label hover:bg-g1 ${halted() ? "text-accent" : "text-text-quiet hover:text-alert"}`}
          onClick={() => command(halted() ? release("city") : halt("city"))}
        >
          {halted() ? say("city_release") : say("city_stop")}
        </button>
      </div>
    </div>
  );
}
