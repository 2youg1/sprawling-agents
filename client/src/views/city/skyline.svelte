<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city, drawn as a skyline at night: one tower per building along
  // one avenue, City Hall at its centre under a dome, a window per run
  // that lights while the run is going, a figure at the door for each
  // run still moving, a flag on the roof where a pursuit stands, a lamp
  // by the door where something is blocked. Every tower carries its
  // name and how far its plan has got; what the glyphs mean is the
  // legend's job, beside the drawing. A click picks a tower, and the
  // panel beside the drawing says what was picked.
  //
  // **This file is the outermost drawing component, so every id a
  // gradient needs is declared once in the `defs` below and referenced
  // by name** (client-SPEC 4-12). Nothing else in the drawing spells
  // `glow` or `ground` a second time.

  import type { RunBelief } from "../../core/belief";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, BuildingProgress, CityAnswer, PursuitLine } from "../../wire";
  import Mark from "./marks.svelte";
  import { squircle } from "./shape";

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

  interface SkylineProps {
    readonly city: CityAnswer;
    readonly picked: Address | null;
    readonly onPick: (addr: Address) => void;
  }

  // One window of a tower, already placed and already lit or dark: a
  // window's slot in the facade is its identity, because a window is
  // built with the tower and is never replaced by its neighbour.
  interface Cell {
    readonly slot: number;
    readonly x: number;
    readonly y: number;
    readonly tone: string;
  }

  interface Tower {
    readonly building: BuildingProgress;
    readonly hall: boolean;
    readonly x: number;
    readonly w: number;
    readonly h: number;
    readonly top: number;
    readonly door: { readonly x: number; readonly y: number; readonly w: number; readonly h: number };
    readonly cells: readonly Cell[];
    readonly active: readonly RunBelief[];
    readonly stuck: number;
    readonly pursuit: PursuitLine | undefined;
    // How much of the plan is done and how much is stuck, as the plinth
    // bar draws it. `null` for a building with no plan: no
    // denominator, so no share - rather than zero, which would read as
    // a plan nobody has started.
    readonly bar: { readonly done: number; readonly blocked: number } | null;
    readonly name: string;
    readonly line: string;
    readonly label: string;
    readonly lamp: string;
  }

  interface Drawing {
    readonly towers: readonly Tower[];
    readonly width: number;
    readonly ground: number;
    readonly height: number;
  }

  interface Star {
    readonly slot: number;
    readonly x: number;
    readonly y: number;
    readonly r: number;
    readonly delay: number;
  }

  const { city, picked, onPick }: SkylineProps = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  function ratio(building: BuildingProgress): { done: number; blocked: number } | null {
    if ("planned" in building.progress) {
      const plan = building.progress.planned;
      return { done: plan.done_ppb / 1e9, blocked: plan.blocked_ppb / 1e9 };
    }
    return null;
  }

  // How tall a building stands: two floors for an empty one, a floor
  // per four runs it has held, never past eight.
  function floorsFor(runs: number, hall: boolean): number {
    const base = hall ? 3 : 2;
    return Math.min(8, base + Math.ceil(runs / 4));
  }

  function windowTone(run: RunBelief | undefined): string {
    if (run === undefined) return "fill-drawn-hollow";
    switch (run.doing.kind) {
      case "frozen":
        return "fill-drawn-line";
      case "waiting":
        return "fill-alert";
      case "calling":
        return "fill-accent-solid blink";
      case "thinking":
        return "fill-accent-solid";
      // A live run whose phase this page was never told: the window is
      // lit, without the blink that says a model is thinking.
      case "unknown":
        return "fill-accent-solid";
    }
  }

  function towerOf(
    place: {
      readonly building: BuildingProgress;
      readonly hall: boolean;
      readonly x: number;
      readonly w: number;
      readonly h: number;
      readonly floors: number;
      readonly cols: number;
      readonly runs: readonly RunBelief[];
    },
    ground: number,
  ): Tower {
    const top = ground - place.h;
    const active = place.runs.filter((run) => run.doing.kind !== "frozen");
    const bar = ratio(place.building);
    const stuck = place.building.blocked.length;
    const cells: Cell[] = [];
    const cellW = (place.w - 24) / place.cols;
    // The hall's ground floor is its colonnade, so its windows stop a
    // floor short.
    const slots = (place.floors - (place.hall ? 1 : 0)) * place.cols;
    for (let slot = 0; slot < slots; slot += 1) {
      const col = slot % place.cols;
      const row = Math.floor(slot / place.cols);
      cells.push({
        slot,
        x: place.x + 12 + col * cellW + cellW / 2 - 6,
        y: top + 18 + row * FLOOR,
        tone: windowTone(place.runs[place.runs.length - 1 - slot]),
      });
    }
    const runs = String(place.runs.length);
    const working = String(active.length);
    const percent = bar === null ? "" : String(Math.round(bar.done * 100));
    let line = "";
    if (bar === null) {
      if (place.runs.length > 0) line = fill(say($lang, "city_tower_runs"), { active: working, runs });
    } else if (place.runs.length === 0) {
      line = fill(say($lang, "city_done_percent"), { percent });
    } else {
      line = fill(say($lang, "city_tower_line"), { active: working, runs, percent });
    }
    return {
      building: place.building,
      hall: place.hall,
      x: place.x,
      w: place.w,
      h: place.h,
      top,
      door: { x: place.x + place.w / 2 - 8, y: ground - 22, w: 16, h: 22 },
      cells,
      active,
      stuck,
      pursuit: city.pursuits.find((each) => each.addr === place.building.addr),
      bar,
      name: place.hall ? say($lang, "city_hall") : String(place.building.addr),
      line,
      label: `${String(place.building.addr)} · ${working} ${say($lang, "city_at_work")}`,
      lamp: `${String(stuck)} ${say($lang, "status_blocked")}`,
    };
  }

  // A handful of stars, placed by a fixed hash so the sky is the same
  // sky on every visit.
  function stars(width: number): Star[] {
    const out: Star[] = [];
    const n = Math.max(12, Math.floor(width / 40));
    for (let i = 0; i < n; i += 1) {
      const a = ((i * 7919) % 1000) / 1000;
      const b = ((i * 104729) % 1000) / 1000;
      out.push({
        slot: i,
        x: 12 + a * (width - 24),
        y: 8 + b * (SKY - 20),
        r: 0.8 + ((i * 31) % 3) * 0.4,
        delay: (i * 137) % 1800,
      });
    }
    return out;
  }

  const drawing = $derived.by((): Drawing => {
    const runs = Object.values($belief.runs).sort((a, b) => (a.started ?? 0) - (b.started ?? 0));
    const of = (addr: string): RunBelief[] =>
      runs.filter((run) => run.addr !== null && (run.addr === addr || run.addr.startsWith(`${addr}/`)));
    const hall = city.buildings.find((each) => each.addr === "hall");
    const others = city.buildings
      .filter((each) => each.addr !== "hall")
      .sort((a, b) => a.addr.localeCompare(b.addr));
    // The hall in the middle, the others alternating to its right and
    // left in name order, so the avenue reads outward from the centre.
    const left: BuildingProgress[] = [];
    const right: BuildingProgress[] = [];
    others.forEach((each, index) => (index % 2 === 0 ? right : left).push(each));
    const row: { readonly building: BuildingProgress; readonly hall: boolean }[] = [
      ...left.reverse().map((building) => ({ building, hall: false })),
      ...(hall === undefined ? [] : [{ building: hall, hall: true }]),
      ...right.map((building) => ({ building, hall: false })),
    ];
    const span = row.reduce((sum, each) => sum + (each.hall ? HALL_W : W) + GAP, -GAP);
    let x = Math.max(24, (MIN_WIDTH - span) / 2);
    const placed = row.map((each) => {
      const held = of(each.building.addr);
      const floors = floorsFor(held.length, each.hall);
      const w = each.hall ? HALL_W : W;
      const slot = {
        building: each.building,
        hall: each.hall,
        x,
        w,
        h: floors * FLOOR + 30,
        floors,
        cols: each.hall ? HALL_COLS : COLS,
        runs: held,
      };
      x += w + GAP;
      return slot;
    });
    const width = Math.max(MIN_WIDTH, ...placed.map((each) => each.x + each.w + 24));
    const tallest = Math.max(120, ...placed.map((each) => each.h));
    const ground = SKY + tallest + 48;
    return {
      towers: placed.map((each) => towerOf(each, ground)),
      width,
      ground,
      height: ground + GROUND_DEPTH,
    };
  });

  const sky = $derived(stars(drawing.width));

  // The drawing is a fixed box with a fixed ratio: a width read from
  // the content and a height read from that width let the scrollbar
  // appear, take width away, shorten the drawing, remove the scrollbar
  // and start again. Nothing here says the city is stopped: the shell's
  // banner says it once, in words.
</script>

<svg
  viewBox="0 0 {drawing.width} {drawing.height}"
  class="block w-full"
  style="aspect-ratio: {drawing.width} / {drawing.height}; max-width: {drawing.width}px"
  role="img"
  aria-label={say($lang, "city_drawing")}
>
  <defs>
    <radialGradient id="glow" cx="50%" cy="60%" r="60%">
      <stop offset="0%" class="[stop-color:var(--color-accent)]" stop-opacity="0.16" />
      <stop offset="100%" class="[stop-color:var(--color-accent)]" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" class="[stop-color:var(--color-drawn-solid)]" />
      <stop offset="100%" class="[stop-color:var(--color-drawn-hollow)]" />
    </linearGradient>
  </defs>
  {#each sky as star (star.slot)}
    <circle cx={star.x} cy={star.y} r={star.r} class="fill-drawn-part blink"
      style="animation-delay: {star.delay}ms; animation-duration: 2.6s" />
  {/each}
  <rect x="0" y={drawing.ground} width={drawing.width} height={GROUND_DEPTH} fill="url(#ground)" />
  <line x1="0" y1={drawing.ground} x2={drawing.width} y2={drawing.ground} class="stroke-drawn-line" stroke-width="1" />
  {#each drawing.towers as tower (tower.building.addr)}
    <g
      class="group cursor-pointer"
      role="link"
      tabindex="0"
      aria-label={tower.label}
      onclick={() => {
        onPick(tower.building.addr);
      }}
      onkeydown={(event) => {
        if (event.key === "Enter") onPick(tower.building.addr);
      }}
    >
      {#if tower.active.length > 0}
        <rect x={tower.x - 16} y={tower.top - 16} width={tower.w + 32} height={tower.h + 16}
          rx="24" fill="url(#glow)" class="transition-opacity" />
      {/if}
      {#if tower.hall}
        <path d="M{tower.x + tower.w / 2 - 34} {tower.top} a34 26 0 0 1 68 0 z"
          class="fill-drawn-solid-lit stroke-drawn-edge" stroke-width="1" />
        <line x1={tower.x + tower.w / 2} y1={tower.top - 26} x2={tower.x + tower.w / 2} y2={tower.top - 38}
          class="stroke-drawn-stem" stroke-width="1.2" />
        <circle cx={tower.x + tower.w / 2} cy={tower.top - 40} r="2" class="fill-drawn-aside" />
      {/if}
      <path
        d={squircle({ x: tower.x, y: tower.top, w: tower.w, h: tower.h }, tower.hall ? 10 : 6, 4)}
        class={[
          tower.hall ? "fill-drawn-solid-lit" : "fill-drawn-solid",
          tower.building.addr === picked ? "stroke-accent" : "stroke-drawn-line",
          "transition-colors group-hover:fill-drawn-line",
        ]}
        stroke-width={tower.building.addr === picked ? "2" : "1"}
      />
      <line x1={tower.x + 8} y1={tower.top + 10} x2={tower.x + tower.w - 8} y2={tower.top + 10}
        class="stroke-drawn-line" stroke-width="1" />
      {#each tower.cells as cell (cell.slot)}
        <rect x={cell.x} y={cell.y} width="12" height="14" rx="1.5" class="{cell.tone} transition-colors" />
      {/each}
      {#if tower.hall}
        {#each [0, 1, 2, 3] as column (column)}
          <rect x={tower.x + tower.w / 2 - 42 + column * 28 - 2.5} y={drawing.ground - 46}
            width="5" height="46" rx="1" class="fill-drawn-line" />
        {/each}
      {/if}
      <rect x={tower.door.x} y={tower.door.y} width={tower.door.w} height={tower.door.h} rx="2" class="fill-drawn-hollow" />
      <rect x={tower.door.x + 2} y={tower.door.y + 2} width={tower.door.w - 4} height="4" rx="1"
        class={tower.active.length > 0 ? "fill-accent-solid" : "fill-drawn-solid-lit"} />
      {#if tower.stuck > 0}
        <g class="blink" aria-label={tower.lamp}>
          <line x1={tower.door.x - 10} y1={drawing.ground} x2={tower.door.x - 10} y2={drawing.ground - 26}
            class="stroke-drawn-part" stroke-width="1" />
          <circle cx={tower.door.x - 10} cy={drawing.ground - 28} r="3" class="fill-alert" />
        </g>
      {/if}
      {#if tower.pursuit !== undefined}
        <Mark placed={{ kind: "flag", x: tower.x + tower.w - 14, y: tower.top - (tower.hall ? 2 : 0), line: tower.pursuit }} />
      {/if}
      <rect x={tower.x} y={drawing.ground + 2} width={tower.w} height={PLINTH} rx="2" class="fill-drawn-solid" />
      {#if tower.bar !== null}
        <rect x={tower.x + 6} y={drawing.ground + 7} width={tower.w - 12} height="3" rx="1.5" class="fill-drawn-line" />
        <rect x={tower.x + 6} y={drawing.ground + 7} width={(tower.w - 12) * tower.bar.done} height="3" rx="1.5" class="fill-accent" />
        <rect x={tower.x + 6 + (tower.w - 12) * tower.bar.done} y={drawing.ground + 7}
          width={(tower.w - 12) * tower.bar.blocked} height="3" rx="1.5" class="fill-alert" />
      {/if}
      <text x={tower.x + tower.w / 2} y={drawing.ground + PLINTH + 22} text-anchor="middle"
        class={["font-mono group-hover:fill-text", tower.building.addr === picked ? "fill-text" : "fill-text-quiet"]}
        font-size="12">
        {tower.name}
      </text>
      {#if tower.line !== ""}
        <text x={tower.x + tower.w / 2} y={drawing.ground + PLINTH + 38} text-anchor="middle"
          class="fill-text-disabled font-mono" font-size="11">
          {tower.line}
        </text>
      {/if}
      {#each tower.active.slice(0, 3) as run, index (run.run)}
        <Mark placed={{ kind: "figure", run, x: tower.door.x + tower.door.w + 12 + index * 16, y: drawing.ground, delay: index * 450 }} />
      {/each}
    </g>
  {/each}
</svg>
