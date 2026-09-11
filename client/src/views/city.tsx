// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city, drawn as a skyline at night: one tower per building along
// one avenue, City Hall at its centre under a dome, a window per run
// that lights while the run is going, a figure at the door for each
// run still moving, a flag on the roof where a pursuit stands, a lamp
// by the door where something is blocked. Nothing is explained on the
// page; a glance says where work moves and where it waits, and a click
// goes inside. One control, the brake.

import { For, Show, createMemo } from "solid-js";

import type { RunBelief } from "../core/belief";
import { halt, release } from "../core/commands";
import type { BuildingProgress, PursuitLine } from "../wire";
import { useCommand, useGo, useSay, useUi } from "../ui";
import { squircle } from "./city/shape";

const W = 128;
const HALL_W = 196;
const GAP = 40;
const FLOOR = 24;
const COLS = 4;
const HALL_COLS = 6;
const PLINTH = 14;
const SKY = 72;
const GROUND_DEPTH = 64;
const MIN_WIDTH = 880;

interface Tower {
  readonly building: BuildingProgress;
  readonly hall: boolean;
  readonly x: number;
  readonly w: number;
  readonly h: number;
  readonly floors: number;
  readonly cols: number;
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

// How tall a building stands: two floors for an empty one, a floor per
// four runs it has held, never past eight.
function floorsFor(runs: number, hall: boolean): number {
  const base = hall ? 3 : 2;
  return Math.min(8, base + Math.ceil(runs / 4));
}

function Figure(props: { readonly run: RunBelief; readonly x: number; readonly y: number; readonly delay: number }) {
  const posture = () => props.run.doing.kind;
  const tone = () => (posture() === "waiting" ? "fill-alert" : "fill-g9");
  return (
    <g
      class="bob"
      style={{ "animation-delay": `${String(props.delay)}ms`, "transform-origin": `${String(props.x)}px ${String(props.y)}px` }}
      aria-label={`${props.run.addr ?? ""} · ${posture()}`}
    >
      <path d={`M${String(props.x - 5)} ${String(props.y)} q5 -13 10 0 z`} class={tone()} />
      <circle cx={props.x} cy={props.y - 16} r="4.2" class={tone()} />
      <Show when={posture() === "thinking"}>
        <g class="blink">
          <circle cx={props.x + 7} cy={props.y - 24} r="1.2" class="fill-g7" />
          <circle cx={props.x + 10.5} cy={props.y - 28} r="1.6" class="fill-g7" />
          <circle cx={props.x + 15} cy={props.y - 33} r="2.2" class="fill-g7" />
        </g>
      </Show>
      <Show when={posture() === "calling"}>
        <rect x={props.x + 5.5} y={props.y - 11} width="6" height="6" rx="1.2" class="fill-accent" />
      </Show>
      <Show when={posture() === "waiting"}>
        <circle cx={props.x} cy={props.y - 16} r="8" class="fill-none stroke-alert" stroke-width="1.4" />
      </Show>
    </g>
  );
}

function Flag(props: { readonly x: number; readonly y: number; readonly line: PursuitLine }) {
  const running = () => props.line.state === "running";
  return (
    <g aria-label={props.line.goal}>
      <line x1={props.x} y1={props.y - 22} x2={props.x} y2={props.y} class="stroke-g6" stroke-width="1" />
      <path
        d={`M${String(props.x)} ${String(props.y - 22)} l16 4 l-16 5 z`}
        class={running() ? "fill-accent wave" : "fill-g5"}
        style={{ "transform-origin": `${String(props.x)}px ${String(props.y - 18)}px` }}
      />
    </g>
  );
}

// The tower is drawn inside a component, where the compiler cannot see
// the `<svg>` above it, so every element is one it knows is SVG by
// name, and the link is a `<g>` with a handler (client-SPEC D47).
function Building(props: { readonly tower: Tower; readonly ground: number }) {
  const say = useSay();
  const go = useGo();
  const t = () => props.tower;
  const top = () => props.ground - t().h;
  const active = createMemo(() => t().runs.filter((run) => run.doing.kind !== "frozen"));
  const bars = createMemo(() => ratio(t().building));
  const stuck = () => t().building.blocked.length;
  // The hall's ground floor is its colonnade, so its windows stop a
  // floor short.
  const cells = createMemo(() => {
    const out: number[] = [];
    for (let i = 0; i < (t().floors - (t().hall ? 1 : 0)) * t().cols; i += 1) out.push(i);
    return out;
  });
  const cell = () => ({ w: (t().w - 24) / t().cols, h: FLOOR });
  const lit = (index: number): RunBelief | undefined => t().runs[t().runs.length - 1 - index];
  const windowClass = (index: number) => {
    const run = lit(index);
    if (run === undefined) return "fill-g0";
    switch (run.doing.kind) {
      case "frozen":
        return "fill-g3";
      case "waiting":
        return "fill-alert";
      case "calling":
        return "fill-accent-solid blink";
      case "thinking":
        return "fill-accent-solid";
    }
  };
  const door = () => ({ x: t().x + t().w / 2 - 8, y: props.ground - 22, w: 16, h: 22 });
  const open = () => {
    go({ kind: "building", address: t().building.addr });
  };

  return (
    <g
      class="group cursor-pointer"
      role="link"
      tabindex="0"
      aria-label={`${t().building.addr} · ${String(active().length)} ${say("city_at_work")}`}
      onClick={open}
      onKeyDown={(event) => {
        if (event.key === "Enter") open();
      }}
    >
      <Show when={active().length > 0}>
        <rect
          x={t().x - 16}
          y={top() - 16}
          width={t().w + 32}
          height={t().h + 16}
          rx="24"
          fill="url(#glow)"
          class="transition-opacity"
        />
      </Show>
      <Show when={t().hall}>
        <path
          d={`M${String(t().x + t().w / 2 - 34)} ${String(top())} a34 26 0 0 1 68 0 z`}
          class="fill-g2 stroke-g4"
          stroke-width="1"
        />
        <line x1={t().x + t().w / 2} y1={top() - 26} x2={t().x + t().w / 2} y2={top() - 38} class="stroke-g6" stroke-width="1.2" />
        <circle cx={t().x + t().w / 2} cy={top() - 40} r="2" class="fill-g7" />
      </Show>
      <path
        d={squircle({ x: t().x, y: top(), w: t().w, h: t().h }, t().hall ? 10 : 6, 4)}
        class={`${t().hall ? "fill-g2" : "fill-g1"} stroke-g3 transition-colors group-hover:fill-g3`}
        stroke-width="1"
      />
      <line x1={t().x + 8} y1={top() + 10} x2={t().x + t().w - 8} y2={top() + 10} class="stroke-g3" stroke-width="1" />
      <For each={cells()}>
        {(index) => {
          const col = () => index % t().cols;
          const row = () => Math.floor(index / t().cols);
          return (
            <rect
              x={t().x + 12 + col() * cell().w + cell().w / 2 - 6}
              y={top() + 18 + row() * cell().h}
              width="12"
              height="14"
              rx="1.5"
              class={`${windowClass(index)} transition-colors`}
            />
          );
        }}
      </For>
      <Show when={t().hall}>
        <For each={[0, 1, 2, 3]}>
          {(i) => (
            <rect
              x={t().x + t().w / 2 - 42 + i * 28 - 2.5}
              y={props.ground - 46}
              width="5"
              height="46"
              rx="1"
              class="fill-g3"
            />
          )}
        </For>
      </Show>
      <rect x={door().x} y={door().y} width={door().w} height={door().h} rx="2" class="fill-g0" />
      <rect
        x={door().x + 2}
        y={door().y + 2}
        width={door().w - 4}
        height="4"
        rx="1"
        class={active().length > 0 ? "fill-accent-solid" : "fill-g2"}
      />
      <Show when={stuck() > 0}>
        <g class="blink" aria-label={`${String(stuck())} ${say("status_blocked")}`}>
          <line x1={door().x - 10} y1={props.ground} x2={door().x - 10} y2={props.ground - 26} class="stroke-g5" stroke-width="1" />
          <circle cx={door().x - 10} cy={props.ground - 28} r="3" class="fill-alert" />
        </g>
      </Show>
      <Show when={t().pursuit}>{(line) => <Flag x={t().x + t().w - 14} y={top() - (t().hall ? 2 : 0)} line={line()} />}</Show>
      <rect x={t().x} y={props.ground + 2} width={t().w} height={PLINTH} rx="2" class="fill-g1" />
      <rect x={t().x + 6} y={props.ground + 7} width={t().w - 12} height="3" rx="1.5" class="fill-g3" />
      <rect x={t().x + 6} y={props.ground + 7} width={(t().w - 12) * bars().done} height="3" rx="1.5" class="fill-accent" />
      <rect
        x={t().x + 6 + (t().w - 12) * bars().done}
        y={props.ground + 7}
        width={(t().w - 12) * bars().blocked}
        height="3"
        rx="1.5"
        class="fill-alert"
      />
      <text
        x={t().x + t().w / 2}
        y={props.ground + PLINTH + 22}
        text-anchor="middle"
        class="fill-text-quiet font-mono group-hover:fill-text"
        font-size="12"
      >
        {t().hall ? say("city_hall") : t().building.addr}
      </text>
      <Show when={t().runs.length > 0}>
        <text
          x={t().x + t().w / 2}
          y={props.ground + PLINTH + 38}
          text-anchor="middle"
          class="fill-text-disabled font-mono"
          font-size="11"
        >
          {active().length > 0 ? `${String(active().length)} / ${String(t().runs.length)}` : String(t().runs.length)}
        </text>
      </Show>
      <For each={active().slice(0, 3)}>
        {(run, index) => <Figure run={run} x={door().x + door().w + 12 + index() * 16} y={props.ground} delay={index() * 450} />}
      </For>
    </g>
  );
}

// A handful of stars, placed by a fixed hash so the sky is the same
// sky on every visit.
function stars(width: number): { x: number; y: number; r: number; delay: number }[] {
  const out: { x: number; y: number; r: number; delay: number }[] = [];
  const n = Math.max(12, Math.floor(width / 40));
  for (let i = 0; i < n; i += 1) {
    const a = ((i * 7919) % 1000) / 1000;
    const b = ((i * 104729) % 1000) / 1000;
    out.push({ x: 12 + a * (width - 24), y: 8 + b * (SKY - 20), r: 0.8 + ((i * 31) % 3) * 0.4, delay: (i * 137) % 1800 });
  }
  return out;
}

export function City() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const city = ui.conn.asking.ask("city_view");

  const towers = createMemo<Tower[]>(() => {
    const answer = city();
    if (answer === undefined || !("city" in answer)) return [];
    const runs = Object.values(ui.conn.belief.runs).sort((a, b) => (a.started ?? 0) - (b.started ?? 0));
    const pursuits = answer.city.pursuits;
    const of = (addr: string) => runs.filter((run) => run.addr !== null && (run.addr === addr || run.addr.startsWith(`${addr}/`)));
    const hall = answer.city.buildings.find((b) => b.addr === "hall");
    const others = answer.city.buildings.filter((b) => b.addr !== "hall").sort((a, b) => a.addr.localeCompare(b.addr));
    // The hall in the middle, the others alternating to its right and
    // left in name order, so the avenue reads outward from the centre.
    const left: BuildingProgress[] = [];
    const right: BuildingProgress[] = [];
    others.forEach((b, i) => (i % 2 === 0 ? right : left).push(b));
    const row: { building: BuildingProgress; hall: boolean }[] = [
      ...left.reverse().map((building) => ({ building, hall: false })),
      ...(hall === undefined ? [] : [{ building: hall, hall: true }]),
      ...right.map((building) => ({ building, hall: false })),
    ];
    const span = row.reduce((sum, { hall: isHall }) => sum + (isHall ? HALL_W : W) + GAP, -GAP);
    let x = Math.max(24, (MIN_WIDTH - span) / 2);
    return row.map(({ building, hall: isHall }) => {
      const held = of(building.addr);
      const floors = floorsFor(held.length, isHall);
      const w = isHall ? HALL_W : W;
      const tower: Tower = {
        building,
        hall: isHall,
        x,
        w,
        h: floors * FLOOR + 30,
        floors,
        cols: isHall ? HALL_COLS : COLS,
        runs: held,
        pursuit: pursuits.find((line) => line.addr === building.addr),
      };
      x += w + GAP;
      return tower;
    });
  });
  const width = createMemo(() => Math.max(MIN_WIDTH, ...towers().map((t) => t.x + t.w + 24)));
  const tallest = createMemo(() => Math.max(120, ...towers().map((t) => t.h)));
  const ground = createMemo(() => SKY + tallest() + 48);
  const height = createMemo(() => ground() + GROUND_DEPTH);
  const halted = () => ui.conn.belief.halted.includes("city");
  const active = createMemo(() => Object.values(ui.conn.belief.runs).filter((run) => run.doing.kind !== "frozen").length);

  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <div class="mx-auto flex w-full max-w-page items-baseline justify-between px-pane pt-wide">
        <h1 class="text-title font-title">{ui.conn.belief.city ?? say("nav_city")}</h1>
        <p class="font-mono text-note text-text-faint">{active() > 0 ? say("city_active", { n: String(active()) }) : say("city_quiet")}</p>
      </div>
      <div class="flex min-h-0 flex-1 flex-col justify-center overflow-auto px-pane py-wide">
        <Show
          when={towers().length > 0}
          fallback={<p class="mx-auto max-w-measure text-center text-text-faint">{say("city_no_buildings")}</p>}
        >
          {/* The drawing is a fixed box with a fixed ratio: a width
              read from the content and a height read from that width
              let the scrollbar appear, take width away, shorten the
              drawing, remove the scrollbar and start again. Nothing
              here says the city is stopped: the shell's banner says it
              once, in words. */}
          <svg
            viewBox={`0 0 ${String(width())} ${String(height())}`}
            class="mx-auto block w-full"
            style={{ "aspect-ratio": `${String(width())} / ${String(height())}`, "max-width": `${String(width())}px` }}
            role="img"
            aria-label={say("city_drawing")}
          >
            <defs>
              <radialGradient id="glow" cx="50%" cy="60%" r="60%">
                <stop offset="0%" class="[stop-color:var(--color-accent)]" stop-opacity="0.16" />
                <stop offset="100%" class="[stop-color:var(--color-accent)]" stop-opacity="0" />
              </radialGradient>
              <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" class="[stop-color:var(--color-g1)]" />
                <stop offset="100%" class="[stop-color:var(--color-g0)]" />
              </linearGradient>
            </defs>
            <For each={stars(width())}>
              {(star) => (
                <circle
                  cx={star.x}
                  cy={star.y}
                  r={star.r}
                  class="fill-g5 blink"
                  style={{ "animation-delay": `${String(star.delay)}ms`, "animation-duration": "2.6s" }}
                />
              )}
            </For>
            <rect x="0" y={ground()} width={width()} height={GROUND_DEPTH} fill="url(#ground)" />
            <line x1="0" y1={ground()} x2={width()} y2={ground()} class="stroke-g3" stroke-width="1" />
            <For each={towers()}>{(tower) => <Building tower={tower} ground={ground()} />}</For>
          </svg>
        </Show>
      </div>
      <div class="mx-auto flex w-full max-w-page items-center justify-end px-pane pb-pane">
        <button
          type="button"
          class={`rounded-control px-base py-tight font-mono text-label hover:bg-g1 ${halted() ? "text-accent" : "text-text-quiet hover:text-alert"}`}
          onClick={() => command(halted() ? release("city") : halt("city"))}
        >
          {halted() ? say("city_release") : say("city_stop")}
        </button>
      </div>
    </div>
  );
}
