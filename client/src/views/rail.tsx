// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Everything that is a button or a badge lives on one edge, so the rest
// of the page has nothing on it but the work. Collapsed, the rail is a
// column of glyphs; expanded - by hover, by `[`, or by `?` - it shows
// each glyph's name and, beside it, the keys that reach it. That is how
// the shortcuts are taught: not up front, but the moment somebody
// looks.

import { For, Show, createMemo } from "solid-js";
import type { JSX } from "solid-js";

import { MAYOR, toFragment } from "../core/route";
import type { View } from "../core/route";
import { useSay, useUi } from "../ui";

export interface RailProps {
  readonly view: View;
  readonly open: boolean;
  readonly onToggle: () => void;
  readonly onPalette: () => void;
}

interface Item {
  readonly key: "talk" | "city" | "mcp" | "record" | "cost" | "setup";
  readonly view: View;
  readonly label: string;
  readonly keys: string;
  readonly glyph: JSX.Element;
  readonly badge?: number | undefined;
}

const stroke = { fill: "none", stroke: "currentColor", "stroke-width": "1.6", "stroke-linecap": "round", "stroke-linejoin": "round" } as const;

function TalkGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M3 5.5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H9l-4 3v-3H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}
function CityGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M3 17V8l4-2v11M7 17V4l5 2v11M12 17V9l5-2v10M2.5 17h15" />
    </svg>
  );
}
function SetupGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M3 6h8M14 6h3M3 14h3M9 14h8" />
      <circle cx="12" cy="6" r="2" />
      <circle cx="7" cy="14" r="2" />
    </svg>
  );
}
function HandGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M6 10V4.5a1.5 1.5 0 0 1 3 0V9M9 4a1.5 1.5 0 0 1 3 0v5M12 5a1.5 1.5 0 0 1 3 0v6.5c0 3-2 5.5-5 5.5s-4.5-2-6-4.5L3 11a1.4 1.4 0 0 1 2.3-1.5L6 10.5" />
    </svg>
  );
}
function PlugGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M7 3v4M13 3v4M5 7h10v3a5 5 0 0 1-10 0zM10 15v3" />
    </svg>
  );
}
function RecordGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M5 3h10v14H5zM8 7h4M8 10h4M8 13h2" />
    </svg>
  );
}
function CostGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <path d="M3 16l4-6 3 3 4-7 3 4M3 17h14" />
    </svg>
  );
}
function PaletteGlyph() {
  return (
    <svg viewBox="0 0 20 20" class="size-glyph" {...stroke}>
      <circle cx="9" cy="9" r="5" />
      <path d="M13 13l4 4" />
    </svg>
  );
}

export function Rail(props: RailProps) {
  const ui = useUi();
  const say = useSay();
  const active = createMemo(
    () => Object.values(ui.conn.belief.runs).filter((run) => run.doing.kind !== "frozen").length,
  );
  const approvals = ui.conn.asking.ask("approval_queue");
  const waiting = createMemo(() => {
    const answer = approvals();
    return answer !== undefined && "approvals" in answer ? answer.approvals.items.length : 0;
  });
  const link = () => ui.conn.state().kind;
  const dotClass = () =>
    link() === "live" ? "bg-accent" : link() === "refused" ? "bg-alert" : "bg-g5 animate-pulse";
  const linkWord = () =>
    link() === "live" ? say("link_live") : link() === "refused" ? say("link_refused") : say("link_connecting");
  const items = createMemo<Item[]>(() => [
    { key: "talk", view: { kind: "talk", address: MAYOR }, label: say("nav_mayor"), keys: "g m", glyph: <TalkGlyph /> },
    { key: "city", view: { kind: "city" }, label: say("nav_city"), keys: "g c", glyph: <CityGlyph />, badge: active() },
    { key: "mcp", view: { kind: "mcp" }, label: say("nav_mcp"), keys: "g x", glyph: <PlugGlyph /> },
    { key: "record", view: { kind: "record", lens: "ledger" }, label: say("nav_the_record"), keys: "g r", glyph: <RecordGlyph /> },
    { key: "cost", view: { kind: "cost" }, label: say("cost_title"), keys: "g $", glyph: <CostGlyph /> },
    { key: "setup", view: { kind: "setup" }, label: say("nav_settings"), keys: "g s", glyph: <SetupGlyph /> },
  ]);
  const here = (item: Item) => (props.view.kind === item.key ? "page" : undefined);
  const halted = () => ui.conn.belief.halted.includes("city");

  return (
    <nav
      class={`group/rail flex h-full shrink-0 flex-col gap-tight border-r border-g1 bg-g0 py-snug transition-[width] ${props.open ? "w-rail-open" : "w-rail"} hover:w-rail-open`}
      aria-label={say("region_nav")}
      data-open={props.open ? "" : undefined}
    >
      <button
        type="button"
        class="flex h-rail items-center gap-base px-base text-label text-text-quiet hover:text-text"
        onClick={() => {
          props.onToggle();
        }}
        aria-expanded={props.open}
        title={linkWord()}
      >
        <span class={`inline-block size-dot shrink-0 rounded-pill ${dotClass()}`} />
        <span class="hidden truncate group-hover/rail:inline group-data-open/rail:inline">
          {ui.conn.belief.city ?? "sprawling"}
        </span>
        <span class="ml-auto hidden font-mono text-note text-text-disabled group-hover/rail:inline group-data-open/rail:inline">[</span>
      </button>
      <Show when={halted()}>
        <div class="mx-snug rounded-pill bg-alert px-tight py-tight text-center text-note text-g0" title={say("city_stopped_line")}>
          <span class="hidden group-hover/rail:inline group-data-open/rail:inline">{say("city_stopped")}</span>
          <span class="group-hover/rail:hidden group-data-open/rail:hidden">!</span>
        </div>
      </Show>
      <For each={items()}>
        {(item) => (
          <a
            href={toFragment(item.view)}
            aria-current={here(item)}
            class="relative flex h-rail items-center gap-base px-base text-label text-text-faint hover:bg-g1 hover:text-text aria-[current=page]:text-text"
            title={item.label}
          >
            <span class="relative shrink-0">
              {item.glyph}
              <Show when={(item.badge ?? 0) > 0}>
                <span class="absolute -top-tight -right-tight rounded-pill bg-accent px-tight text-note leading-none text-g0">
                  {item.badge}
                </span>
              </Show>
            </span>
            <span class="hidden truncate group-hover/rail:inline group-data-open/rail:inline">{item.label}</span>
            <kbd class="ml-auto hidden font-mono text-note text-text-disabled group-hover/rail:inline group-data-open/rail:inline">
              {item.keys}
            </kbd>
          </a>
        )}
      </For>
      <Show when={waiting() > 0}>
        <a
          href={toFragment({ kind: "talk", address: MAYOR })}
          class="flex h-rail items-center gap-base px-base text-label text-alert hover:bg-g1"
          title={say("nav_waiting", { n: String(waiting()) })}
        >
          <span class="relative shrink-0">
            <HandGlyph />
            <span class="absolute -top-tight -right-tight rounded-pill bg-alert px-tight text-note leading-none text-g0">
              {waiting()}
            </span>
          </span>
          <span class="hidden truncate group-hover/rail:inline group-data-open/rail:inline">
            {say("nav_waiting", { n: String(waiting()) })}
          </span>
          <kbd class="ml-auto hidden font-mono text-note text-text-disabled group-hover/rail:inline group-data-open/rail:inline">g w</kbd>
        </a>
      </Show>
      <span class="flex-1" />
      <button
        type="button"
        class="flex h-rail items-center gap-base px-base text-label text-text-faint hover:bg-g1 hover:text-text"
        onClick={() => {
          props.onPalette();
        }}
        title={say("nav_palette")}
      >
        <PaletteGlyph />
        <span class="hidden truncate group-hover/rail:inline group-data-open/rail:inline">{say("nav_everything")}</span>
        <kbd class="ml-auto hidden font-mono text-note text-text-disabled group-hover/rail:inline group-data-open/rail:inline">⌘K</kbd>
      </button>
    </nav>
  );
}
