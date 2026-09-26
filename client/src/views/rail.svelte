<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Everything that is a button or a badge lives on one edge, so the
  // rest of the page has nothing on it but the work. Three postures
  // (client-SPEC 7E): icons, icons with names, and away - collapsed,
  // by hover, by the accelerator and `B`, or by `?` - showing each
  // glyph's name and, beside it, the keys that reach it. That is how
  // the shortcuts are taught: not up front, but the moment somebody
  // looks. The keys are read from `core/keys`, so a rebind shows here
  // without this file knowing what was pressed.
  //
  // **Five items, because a rail is scanned every day.** The tool
  // servers page stands with the settings page's accounts group ("what
  // the city may reach" is one class of thing), and two top-level
  // entries for one class of thing made the rail speak two taxonomies.
  // The palette and the settings page both still reach it.
  //
  // **Opening on hover is the stylesheet's job, not this file's.**
  // `nav[data-rail]` and `.rail-label` are the two hooks that rule
  // needs (`theme.css`), and a pointer that only crosses the column
  // never starts the transition at all. This file keeps the pinned
  // state, which is a decision rather than a gesture.
  //
  // **The dot at the top is the notice drawer's** (client-SPEC 4-35,
  // 7D): it means one thing - something waits for this person - and its
  // own control opens the drawer that says which. The city's name
  // beside it pins the rail, so neither control answers for the other.

  import type { Rail } from "../core/prefs";
  import type { Action } from "../core/keys";
  import { MAYOR, toFragment } from "../core/route";
  import type { View } from "../core/route";
  import { cityIsShut } from "../core/scope";
  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import Notices from "./notices.svelte";
  import Badge from "./parts/badge.svelte";
  import Glyph from "./parts/glyph.svelte";
  import type { GlyphName } from "./parts/glyph";
  import { Kbd } from "./parts/kbd.svelte";
  import Tip from "./parts/tip.svelte";

  interface RailProps {
    readonly view: View;
    readonly posture: Rail;
    readonly onToggle: () => void;
    readonly onPalette: () => void;
  }

  interface Item {
    readonly key: "talk" | "city" | "record" | "cost" | "setup";
    readonly view: View;
    // Already in the person's language.
    readonly label: string;
    readonly action: Action;
    readonly glyph: GlyphName;
    readonly badge?: number | undefined;
  }

  const { view, posture, onToggle, onPalette }: RailProps = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const approvals = u.approvals;

  // What a name does while the rail is collapsed. Written on every name
  // the rail can show, so one stylesheet rule reveals them all on hover
  // and this file decides only the pinned case.
  const label = $derived(`rail-label truncate ${posture === "named" ? "block" : "hidden"}`);

  const active = $derived($belief.live.length);
  const waiting = $derived($approvals.length);
  const halted = $derived(cityIsShut($belief.halted));

  const items = $derived.by((): Item[] => [
    { key: "talk", view: { kind: "talk", address: MAYOR }, label: say($lang, "nav_mayor"), action: "go.talk", glyph: "talk" },
    { key: "city", view: { kind: "city" }, label: say($lang, "nav_city"), action: "go.city", glyph: "city", badge: active },
    { key: "record", view: { kind: "record", lens: "ledger" }, label: say($lang, "nav_the_record"), action: "go.record", glyph: "record" },
    { key: "cost", view: { kind: "cost" }, label: say($lang, "cost_title"), action: "go.cost", glyph: "cost" },
    { key: "setup", view: { kind: "setup" }, label: say($lang, "nav_settings"), action: "go.setup", glyph: "setup" },
  ]);

  function here(item: Item): "page" | undefined {
    return view.kind === item.key ? "page" : undefined;
  }

  // How wide the column is, and how wide the nav drawn inside it is.
  // Two widths rather than one, because a hover widens the nav over the
  // page instead of pushing it: reading is never disturbed by a pointer
  // crossing the edge. Away is the third posture and takes both to
  // zero - the page gets the whole window, and the only way back is the
  // accelerator, which is why the sheet of keys and the palette both
  // still reach every view.
  const column = $derived.by((): string => {
    switch (posture) {
      case "named":
        return "w-rail-open";
      case "glyphs":
        return "w-rail";
      case "away":
        return "w-0";
    }
  });
</script>

<div class="relative h-full shrink-0 transition-[width] duration-200 motion-reduce:transition-none {column}">
  <nav
    data-rail={posture}
    class={[
      "absolute inset-y-0 left-0 z-10 flex flex-col gap-tight border-r border-edge bg-chrome py-snug",
      "transition-[width] duration-200 motion-reduce:transition-none",
      column,
      posture === "named" ? "shadow-float" : "",
      posture === "away" ? "overflow-hidden border-r-0" : "",
    ]}
    aria-label={say($lang, "region_nav")}
    aria-hidden={posture === "away" ? true : undefined}
  >
    <div class="flex h-rail w-full items-center">
      <Notices {posture} />
      <button
        type="button"
        class="{label} min-w-0 flex-1 pr-base text-left text-label text-text-quiet hover:text-text"
        aria-expanded={posture === "named"}
        onclick={onToggle}
      >
        <span class="flex items-center gap-base">
          <span class="truncate">{$belief.city ?? say($lang, "nav_city")}</span>
          <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
          {@render Kbd({ action: "rail.toggle", class: "ml-auto" })}
        </span>
      </button>
    </div>
    {#if halted}
      <div class="flex h-rail w-full items-center gap-base px-base" role="status" aria-label={say($lang, "halt_title")}>
        <Badge text={say($lang, "halt_title")} weight="alert" dot />
        <span class={label}>{say($lang, "halt_title")}</span>
      </div>
    {/if}
    {#each items as item (item.key)}
      <Tip text={item.label}>
        {#snippet children(hint: string)}
          <a
            href={toFragment(item.view)}
            aria-current={here(item)}
            aria-labelledby={hint}
            class="relative flex h-rail w-full items-center gap-base px-base text-label text-text-faint hover:bg-chrome hover:text-text aria-[current=page]:text-text"
          >
            <span class="relative shrink-0">
              <Glyph name={item.glyph} />
              <!-- A count of what is running is a reading, not an
                   alarm: it lifts a surface rather than spending the
                   accent, which this page keeps for the focus ring and
                   for the bar beside a selected row (7B). The badge on
                   the row below does spend a coloured token, because
                   that one counts people waiting on an answer. -->
              {#if (item.badge ?? 0) > 0}
                <span class="absolute -top-tight -right-tight">
                  <Badge text={String(item.badge ?? 0)} weight="quiet" />
                </span>
              {/if}
            </span>
            <span class="{label} flex-1">{item.label}</span>
            <span class={label}>
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
              {@render Kbd({ action: item.action })}
            </span>
          </a>
        {/snippet}
      </Tip>
    {/each}
    {#if waiting > 0}
      <Tip text={fill(say($lang, "nav_waiting"), { n: String(waiting) })}>
        {#snippet children(hint: string)}
          <a
            href={toFragment({ kind: "talk", address: MAYOR })}
            aria-labelledby={hint}
            class="flex h-rail w-full items-center gap-base px-base text-label text-alert hover:bg-chrome"
          >
            <span class="relative shrink-0">
              <Glyph name="hand" />
              <span class="absolute -top-tight -right-tight">
                <Badge text={String(waiting)} weight="alert" />
              </span>
            </span>
            <span class="{label} flex-1">{fill(say($lang, "nav_waiting"), { n: String(waiting) })}</span>
            <span class={label}>
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
              {@render Kbd({ action: "go.waiting" })}
            </span>
          </a>
        {/snippet}
      </Tip>
    {/if}
    <span class="flex-1"></span>
    <Tip text={say($lang, "nav_palette")}>
      {#snippet children(hint: string)}
        <button
          type="button"
          class="flex h-rail w-full items-center gap-base px-base text-label text-text-faint hover:bg-chrome hover:text-text"
          aria-labelledby={hint}
          onclick={onPalette}
        >
          <Glyph name="search" />
          <span class="{label} flex-1">{say($lang, "nav_everything")}</span>
          <span class={label}>
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
            {@render Kbd({ action: "palette" })}
          </span>
        </button>
      {/snippet}
    </Tip>
  </nav>
</div>
