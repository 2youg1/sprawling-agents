// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Everything that is a button or a badge lives on one edge, so the rest
// of the page has nothing on it but the work. Collapsed, the rail is a
// column of glyphs; expanded - by hover, by `[`, or by `?` - it shows
// each glyph's name and, beside it, the keys that reach it. That is how
// the shortcuts are taught: not up front, but the moment somebody
// looks. The keys themselves are read from `core/keys`, so a rebind
// shows here without this file knowing what was pressed.
//
// The hover has a threshold on the way in and a delay on the way out. A
// pointer crossing the 44 px edge used to widen the rail, which moved
// the edge out from under the pointer, which narrowed it again; a
// pointer must now rest on the column before it opens, and leaving it
// for a moment does not close it.

import { For, Show, createMemo, createSignal, onCleanup } from "solid-js";
import type { JSX } from "solid-js";

import type { Action } from "../core/keys";
import { MAYOR, toFragment } from "../core/route";
import type { View } from "../core/route";
import { useSay, useUi } from "../ui";
import { Kbd } from "./parts/kbd";

// How long a pointer rests on the collapsed column before it opens, and
// how long the rail stays open after the pointer leaves.
const ENTER_MS = 120;
const LEAVE_MS = 200;

export interface RailProps {
  readonly view: View;
  readonly open: boolean;
  // The prefix key already pressed, which the rail says out loud so the
  // second key is not guessed at.
  readonly prefix: string | null;
  readonly onToggle: () => void;
  readonly onPalette: () => void;
}

interface Item {
  readonly key: "talk" | "city" | "mcp" | "record" | "cost" | "setup";
  readonly view: View;
  readonly label: string;
  readonly action: Action;
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

// A dot drawn in the same box a glyph is drawn in, so every row on the
// rail starts its first mark at the same x. It used to be centred in
// its own smaller box, five pixels left of every icon under it.
function Dot(props: { readonly tone: string }) {
  return (
    <span class="flex size-glyph shrink-0 items-center justify-center">
      <span class={`inline-block size-dot rounded-pill ${props.tone}`} />
    </span>
  );
}

export function Rail(props: RailProps) {
  const ui = useUi();
  const say = useSay();
  const [resting, setResting] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const settle = (open: boolean, after: number) => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      setResting(open);
    }, after);
  };
  onCleanup(() => {
    clearTimeout(timer);
  });
  // What the rail shows the names in: pinned open, or a pointer that
  // stayed.
  const wide = () => props.open || resting();

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
    { key: "talk", view: { kind: "talk", address: MAYOR }, label: say("nav_mayor"), action: "go.talk", glyph: <TalkGlyph /> },
    { key: "city", view: { kind: "city" }, label: say("nav_city"), action: "go.city", glyph: <CityGlyph />, badge: active() },
    { key: "mcp", view: { kind: "mcp" }, label: say("nav_mcp"), action: "go.mcp", glyph: <PlugGlyph /> },
    { key: "record", view: { kind: "record", lens: "ledger" }, label: say("nav_the_record"), action: "go.record", glyph: <RecordGlyph /> },
    { key: "cost", view: { kind: "cost" }, label: say("cost_title"), action: "go.cost", glyph: <CostGlyph /> },
    { key: "setup", view: { kind: "setup" }, label: say("nav_settings"), action: "go.setup", glyph: <SetupGlyph /> },
  ]);
  const here = (item: Item) => (props.view.kind === item.key ? "page" : undefined);
  const halted = () => ui.conn.belief.halted.includes("city");

  return (
    // The column the page lays out beside is the collapsed width unless
    // the rail is pinned open; a hover widens the nav over the page
    // rather than pushing it, so reading is never disturbed by the
    // pointer passing the edge.
    <div class={`relative h-full shrink-0 transition-[width] duration-200 motion-reduce:transition-none ${props.open ? "w-rail-open" : "w-rail"}`}>
      <nav
        class={`absolute inset-y-0 left-0 z-10 flex flex-col gap-tight border-r border-g1 bg-g0 py-snug transition-[width] duration-200 motion-reduce:transition-none ${wide() ? "w-rail-open shadow-composer" : "w-rail"}`}
        aria-label={say("region_nav")}
        onPointerEnter={() => {
          settle(true, ENTER_MS);
        }}
        onPointerLeave={() => {
          settle(false, LEAVE_MS);
        }}
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
          <Dot tone={dotClass()} />
          <Show when={wide()}>
            <span class="truncate">{ui.conn.belief.city ?? "sprawling"}</span>
            <Kbd action="rail.toggle" class="ml-auto" />
          </Show>
        </button>
        <Show when={halted()}>
          <div
            class="flex h-rail items-center gap-base px-base text-label text-alert"
            role="status"
            title={say("halt_title")}
          >
            <Dot tone="bg-alert" />
            <Show when={wide()}>
              <span class="truncate">{say("halt_title")}</span>
            </Show>
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
              <Show when={wide()}>
                <span class="truncate">{item.label}</span>
                <Kbd action={item.action} class="ml-auto" />
              </Show>
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
            <Show when={wide()}>
              <span class="truncate">{say("nav_waiting", { n: String(waiting()) })}</span>
              <Kbd action="go.waiting" class="ml-auto" />
            </Show>
          </a>
        </Show>
        <span class="flex-1" />
        <Show when={props.prefix !== null}>
          <div class="mx-snug rounded-control bg-g2 px-snug py-tight text-center font-mono text-note text-text-quiet" role="status">
            {say("keys_prefix", { key: props.prefix ?? "" })}
          </div>
        </Show>
        <button
          type="button"
          class="flex h-rail items-center gap-base px-base text-label text-text-faint hover:bg-g1 hover:text-text"
          onClick={() => {
            props.onPalette();
          }}
          title={say("nav_palette")}
        >
          <PaletteGlyph />
          <Show when={wide()}>
            <span class="truncate">{say("nav_everything")}</span>
            <Kbd action="palette" class="ml-auto" />
          </Show>
        </button>
      </nav>
    </div>
  );
}
