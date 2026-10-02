<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The right side, the inspector (docs/frontend-method.md §7F, client/Spec.lean §4-45): a strip of the
  // items open on it, the editor region above and the terminal region
  // below, and the line between them. Where it stands on the grid is the
  // workspace's decision (4-27); what is open on it is
  // `inspect/open.svelte.ts`'s.
  //
  // **Nobody has to open anything for the run to be inspectable.** While
  // no item is open, the side follows the run in front of the person: the
  // file its last call read or changed above, the command it last ran
  // below (`reading.ts`'s `followingIn`). Choosing one of those tabs keeps
  // them both, as items the person now holds.
  //
  // **Each region shows its own front item, and every open item stays
  // mounted** behind the one shown, so its scroll, its selection and a
  // RefRain draft survive another tab coming forward.
  //
  // **Escape closes the inspector** unless something inside it - an
  // input method, a panel of RefRain's - took the key first, and the focus
  // goes back to the control that opened it (7-7).
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { derived } from "svelte/store";

  import { say } from "../core/lang";
  import { toFragment } from "../core/route";
  import { ui } from "../ui";
  import type { Address, Call, RoundsAnswer } from "../wire";
  import Changes from "./changes.svelte";
  import { commandOf } from "./monitor/trace";
  import Called from "./inspect/called.svelte";
  import {
    INSPECTOR,
    closeItem,
    closeRight,
    frontWhere,
    itemKey,
    openItems,
    rightItem,
    sameItem,
    showItem,
    type RightItem,
  } from "./inspect/open.svelte";
  import { readFrom, readingOf, regionOf, type Region, type Tab } from "./inspect/reading";
  import Split from "./inspect/split.svelte";
  import Strip from "./inspect/strip.svelte";
  import RefRain from "./refrain/refrain.svelte";

  interface Props {
    // What the side shows while nobody has opened an item.
    readonly following: readonly RightItem[];
    // The rounds of the run in front, which the page asked already to
    // know what to follow.
    readonly current: RoundsAnswer | undefined;
    // The conversation beside it, which a quoted line is written into.
    readonly talk: Address;
  }

  const { following, current, talk }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const PANELS: Readonly<Record<Region, string>> = { editor: "inspector-editor", terminal: "inspector-terminal" };

  // The fewest lines either region keeps when the line between them moves.
  const LEAST = 6;

  const held = $derived(openItems());
  const items = $derived(held.length > 0 ? held : following);

  // The rounds of every run an open call belongs to, asked once per run;
  // `asking` merges a question the thread beside it asks as well. One
  // store over all of them, read in the template, so an answer already
  // held draws in the same pass that opened the item.
  const runs = $derived([...new Set(items.flatMap((item) => (item.kind === "call" ? [item.run] : [])))]);
  const asked = $derived(
    derived(
      runs.map((run) => u.conn.asking.ask({ rounds: { run } })),
      (answers) => new Map(answers.flatMap((answer) => (answer !== undefined && "rounds" in answer ? [[answer.rounds.run, answer.rounds] as const] : []))),
    ),
  );
  const rounds = $derived(current === undefined || $asked.has(current.run) ? $asked : new Map([...$asked, [current.run, current]]));

  // An open call as its run's rounds hold it: `null` until the rounds are
  // answered, and a `null` call when they hold no call at that place.
  function callOf(item: RightItem): { readonly rounds: RoundsAnswer; readonly call: Call | null } | null {
    if (item.kind !== "call") return null;
    const answer = rounds.get(item.run);
    if (answer === undefined) return null;
    return { rounds: answer, call: answer.turns.flatMap((turn) => turn.calls).find((call) => call.at === item.at) ?? null };
  }

  function regionOfItem(item: RightItem): Region {
    const call = callOf(item)?.call ?? null;
    return call === null ? "editor" : regionOf(readingOf(call));
  }

  // An oid as git prints it short.
  const SHORT = 7;

  function labelOf(item: RightItem): string {
    if (item.kind === "document") return item.path.slice(item.path.lastIndexOf("/") + 1);
    if (item.kind === "changes") return item.head.slice(0, SHORT);
    const call = callOf(item)?.call ?? null;
    if (call === null) return "…";
    const entry = call.render === "terminal" ? commandOf(call) : null;
    if (entry?.kind === "command") return entry.text;
    const path = readFrom(call)?.path ?? (readingOf(call) === "diff" ? (call.subject ?? "") : "");
    return path === "" ? call.tool : path.slice(path.lastIndexOf("/") + 1);
  }

  const tabs = $derived(items.map((item): Tab => ({ item, label: labelOf(item), region: regionOfItem(item) })));
  const front = $derived(held.length > 0 ? rightItem() : (following[0] ?? null));

  // An item as a link to this conversation with it open on the right
  // (4-63); what moved between two commits has no spelling there.
  function linkOf(item: RightItem): string | null {
    switch (item.kind) {
      case "call":
        return toFragment({ kind: "talk", address: talk, item: { kind: "call", run: item.run, at: item.at } });
      case "document":
        return toFragment({ kind: "talk", address: talk, item: { kind: "document", building: item.building, path: item.path, version: item.version } });
      case "changes":
        return null;
    }
  }

  // What each region shows: the item of that region touched last, or
  // the one the side follows.
  function frontOf(region: Region): RightItem | null {
    return held.length > 0
      ? frontWhere((item) => regionOfItem(item) === region)
      : (following.find((item) => regionOfItem(item) === region) ?? null);
  }
  const editorTabs = $derived(tabs.filter((tab) => tab.region === "editor"));
  const terminalTabs = $derived(tabs.filter((tab) => tab.region === "terminal"));
  const editorFront = $derived(frontOf("editor"));
  const terminalFront = $derived(frontOf("terminal"));

  function pick(item: RightItem): void {
    if (held.length === 0) for (const kept of following) if (!sameItem(item, kept)) showItem(kept);
    showItem(item);
  }

  function close(item: RightItem): void {
    if (held.length === 0) for (const kept of following) if (!sameItem(item, kept)) showItem(kept);
    closeItem(item);
    if (openItems().length === 0) u.prefs.setPanel(false);
  }

  function closeAll(): void {
    closeRight();
    u.prefs.setPanel(false);
  }

  // The line between the regions, counted in lines of the code below it:
  // `null` until somebody moves it, and then the editor's own height.
  let step = $state(24);
  onMount(() => {
    const baseline = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--spacing-baseline"));
    if (Number.isFinite(baseline) && baseline > 0) step = 3 * baseline;
  });
  let chosen = $state<number | null>(null);
  let room = $state(0);
  let editorHeight = $state(0);
  const most = $derived(Math.max(LEAST, Math.floor(room / step) - LEAST));
  const lines = $derived(chosen ?? Math.round(editorHeight / step));
</script>

{#snippet itemView(item: RightItem)}
  {#if item.kind === "document"}
    <div class="min-h-0 flex-1 overflow-auto bg-page">
      <RefRain building={item.building} path={item.path} version={item.version} />
    </div>
  {:else if item.kind === "changes"}
    <div class="min-h-0 flex-1 overflow-auto bg-page px-wide py-snug">
      <Changes base={item.base} head={item.head} {talk} />
    </div>
  {:else}
    {@const found = callOf(item)}
    {#if found === null}
      <p class="px-wide py-snug text-note text-text-faint">…</p>
    {:else if found.call === null}
      <p class="px-wide py-snug text-note text-text-faint">{say($lang, "inspect_call_gone")}</p>
    {:else}
      <Called call={found.call} rounds={found.rounds} {talk} />
    {/if}
  {/if}
{/snippet}

<aside
  {...{ [INSPECTOR]: "" }}
  aria-label={say($lang, "inspect_label")}
  class="side-in flex h-full min-h-0 w-full flex-col bg-chrome"
  onkeydown={(event) => {
    if (event.key !== "Escape" || event.isComposing || event.defaultPrevented) return;
    event.preventDefault();
    closeAll();
  }}
>
  <Strip {tabs} {front} panels={PANELS} {linkOf} onPick={pick} onClose={close} onCloseAll={closeAll} />
  <div class="side-in-then flex min-h-0 flex-1 flex-col" bind:clientHeight={room}>
    {#if editorTabs.length > 0}
      <div
        id={PANELS.editor}
        role="tabpanel"
        aria-label={say($lang, "inspect_editor")}
        class={["flex min-h-0 flex-col bg-page", chosen === null && terminalTabs.length > 0 ? "flex-[3]" : "", terminalTabs.length === 0 ? "flex-1" : "shrink-0"]}
        style:height={chosen === null || terminalTabs.length === 0 ? undefined : `${String(chosen * step)}px`}
        bind:clientHeight={editorHeight}
      >
        {#each editorTabs as tab (itemKey(tab.item))}
          <div class={sameItem(editorFront, tab.item) ? "flex min-h-0 flex-1 flex-col" : "hidden"}>
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render itemView(tab.item)}
          </div>
        {/each}
      </div>
    {/if}
    {#if editorTabs.length > 0 && terminalTabs.length > 0}
      <Split
        lines={Math.min(most, Math.max(LEAST, lines))}
        least={LEAST}
        {most}
        controls={PANELS.editor}
        {step}
        onLines={(next) => {
          chosen = next;
        }}
        onReset={() => {
          chosen = null;
        }}
      />
    {/if}
    {#if terminalTabs.length > 0}
      <div
        id={PANELS.terminal}
        role="tabpanel"
        aria-label={say($lang, "inspect_terminal")}
        class={["flex min-h-0 flex-col", chosen === null && editorTabs.length > 0 ? "flex-[2]" : "flex-1"]}
      >
        {#each terminalTabs as tab (itemKey(tab.item))}
          <div class={sameItem(terminalFront, tab.item) ? "flex min-h-0 flex-1 flex-col" : "hidden"}>
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render itemView(tab.item)}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</aside>
