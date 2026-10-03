<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The world layer's sessions pane (client/Spec.lean §7K): one row per session,
  // the stretches of every room including the past ones, newest first -
  // the pinned above, then grouped by building. A row is a link that makes
  // its session the one in main: the room's current session is talked to,
  // an earlier one is read (`talk/past.svelte`). Beside each row its menu
  // pins it and gives and strips tags (`session_menu.svelte`); a row of the
  // tags in use above the list filters it by one.
  //
  // A row is titled by the session's name, or its room when it has none,
  // and says the model, effort and workspace of its last run and the
  // start of its last reply, cut to the width the row has
  // (`crates/wire/spec/Answer/Sessions.lean` D27).
  //
  // The rows are `core/stretches.ts`'s reading of the city's answers
  // (`stretches.svelte.ts`), and the tags are the person's preferences
  // as the city keeps them (`core/tags.ts`).
  import type { Snippet } from "svelte";

  import type { RunBelief } from "../../core/belief";
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { roomOf, toFragment } from "../../core/route";
  import { grouped, pinningOf } from "../../core/stretches";
  import type { Stretch } from "../../core/stretches";
  import { PIN, inUse, namedIn, tagsOf } from "../../core/tags";
  import type { Named } from "../../core/tags";
  import { ago, lasted } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, Seq, Tag } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import ContextBar from "./context_bar.svelte";
  import { ticker } from "../talk/timing";
  import SessionMenu from "./session_menu.svelte";
  import { askStretches } from "./stretches.svelte";

  interface Props {
    // The room whose session is in main, and which of its sessions:
    // absent is the room's current one.
    readonly here: Address;
    readonly session?: Seq | undefined;
    // Whether the pane is as narrow as the right pane leaves it, which
    // drops the second line and the time.
    readonly narrow: boolean;
    // The pane's label: a menu that moves it, in the panorama tier.
    readonly head: Snippet;
  }

  const { here, session, narrow, head }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const held = u.tags.held;
  const stretches = askStretches(u);

  type State = "run" | "ask" | "done";

  // A past session is done whatever its last run says: the city goes on
  // only with the current one.
  function stateOf(stretch: Stretch): State {
    const last = stretch.runs.at(-1);
    if (!stretch.current || last === undefined) return "done";
    switch (last.doing.kind) {
      case "frozen":
        return "done";
      case "waiting":
        return "ask";
      case "unknown":
      case "thinking":
      case "calling":
        return "run";
    }
  }

  // A running row says how long its run has gone, on the page's one
  // clock: it moves only while a row reads it and the page is seen, and
  // every reading is recomputed from the run's own start (client/Spec.lean §4-59).
  const tick = ticker(u.now);

  const city = $derived($belief.city);
  function named(stretch: Stretch): Named | null {
    return namedIn(city, stretch.room, stretch.line.began, stretch.line.workspace ?? null);
  }
  function tagsFor(stretch: Stretch): readonly Tag[] {
    const name = named(stretch);
    return name === null ? [] : tagsOf($held, name);
  }

  // One tag the pane is filtered by, or every row. A tag nobody carries
  // any more filters nothing.
  let filter = $state<Tag | null>(null);
  const offered = $derived(city === null ? [] : inUse(stretches.all.map(tagsFor)));
  const by = $derived(filter !== null && offered.includes(filter) ? filter : null);
  const groups = $derived(grouped(stretches.all, tagsFor, by));

  function chosen(stretch: Stretch): boolean {
    return stretch.room === here && (session === undefined ? stretch.current : stretch.line.began === session);
  }

  function hrefOf(stretch: Stretch): string {
    return toFragment(
      stretch.current ? { kind: "talk", address: stretch.room } : { kind: "talk", address: stretch.room, session: stretch.line.began },
    );
  }

  function when(stretch: Stretch, state: State): string {
    const started = stretch.runs.at(-1)?.started ?? null;
    switch (state) {
      case "run":
        return started === null ? "" : lasted($tick - started);
      case "ask":
        return say($lang, "world_waiting");
      case "done":
        return ago($lang, stretch.line.at, $tick);
    }
  }

  // What the second line says: the start of the session's last reply,
  // else the task of its last run, else how the session began when this
  // page holds none of its runs.
  function about(stretch: Stretch, last: RunBelief | undefined): string {
    return stretch.line.preview ?? last?.task ?? say($lang, startOf(stretch));
  }

  function titleOf(stretch: Stretch): string {
    const name = stretch.line.name ?? "";
    return name === "" ? roomOf(stretch.room) : name;
  }

  // What the session ran on: model, effort and workspace, each left out
  // when the city does not say it.
  function ranOn(stretch: Stretch): string {
    const { model, effort, workspace } = stretch.line;
    return [model ?? null, effort === undefined || effort === null ? null : say($lang, `effort_${effort}`), workspace ?? null]
      .filter((each) => each !== null && each !== "")
      .join(" · ");
  }

  function startOf(stretch: Stretch): Key {
    const start = stretch.line.start;
    if (start === "dispatched") return "mailbox_start_dispatched";
    if (start.opened.from !== undefined && start.opened.from !== null) return "mailbox_start_forked";
    return start.opened.carry === "handoff" ? "mailbox_start_carried" : "mailbox_start_new";
  }

  const DOT: Record<State, string> = {
    run: "bg-accent shadow-[0_0_0_3px_color-mix(in_oklch,var(--color-accent)_14%,transparent)]",
    ask: "bg-alert shadow-[0_0_0_3px_color-mix(in_oklch,var(--color-alert)_12%,transparent)]",
    done: "border-[1.5px] border-mark",
  };
</script>

<section class="flex min-h-0 flex-1 flex-col overflow-hidden px-snug" aria-label={say($lang, "world_sessions")}>
  {@render head()}
  {#if offered.length > 0}
    <div class="mb-snug flex flex-wrap gap-tight" role="group" aria-label={say($lang, "world_tags")}>
      {#each [null, ...offered] as tag (tag ?? "")}
        <button
          type="button"
          class="h-control-sm rounded-pill px-snug text-note text-text-quiet hover:wash hover:text-text aria-pressed:wash-strong aria-pressed:text-text"
          aria-pressed={by === tag}
          onclick={() => {
            filter = by === tag ? null : tag;
          }}
        >
          {tag ?? say($lang, "world_tags_all")}
        </button>
      {/each}
    </div>
  {/if}
  <!-- A row reaches out by one step on each side, so its wash and its
  chosen bar stand outside the text; the column that scrolls is widened
  by that step into the pane's own inset, so the pane clips neither. -->
  <div class="-mx-snug min-h-0 flex-1 overflow-y-auto px-snug">
    {#each groups as group (group.kind === "pinned" ? "" : group.building)}
      <h3 class="mt-base mb-tight text-note text-text-faint first:mt-0">
        {group.kind === "pinned" ? say($lang, "world_pinned") : group.building}
      </h3>
      <ul>
        {#each group.rows as stretch (`${stretch.room}:${String(stretch.line.began)}`)}
          {@const state = stateOf(stretch)}
          {@const last = stretch.runs.at(-1)}
          {@const tags = tagsFor(stretch)}
          {@const pinning = pinningOf(stretch, tags)}
          {@const inMain = chosen(stretch)}
          <li class={["group relative -mx-snug flex items-start rounded-card", inMain ? "wash-strong" : "hover:wash"]}>
            <a
              href={hrefOf(stretch)}
              class="grid min-w-0 flex-1 grid-cols-[12px_minmax(0,1fr)_auto] gap-x-base rounded-card px-snug py-snug"
              aria-current={inMain ? "page" : undefined}
            >
              {#if inMain}
                <span class="absolute top-[10px] bottom-[10px] left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
              {/if}
              <span class={["mt-snug size-dot rounded-pill", DOT[state]]} aria-hidden="true"></span>
              <span class="flex min-w-0 items-center gap-tight">
                <span class={["truncate font-label", stretch.current ? "text-text" : "text-text-quiet"]} title={stretch.room}>{titleOf(stretch)}</span>
                {#if pinning !== "none"}
                  <span class="shrink-0 text-text-faint" title={pinning === "mayor" ? say($lang, "world_pinned_mayor") : undefined}>
                    <Glyph name="pin" size="sm" />
                  </span>
                {/if}
              </span>
              {#if !narrow}
                <span class="figure text-note text-text-faint">{when(stretch, state)}</span>
                <span class="col-start-2 col-end-4 line-clamp-2 text-note text-text-quiet">{about(stretch, last)}</span>
                {#if ranOn(stretch) !== ""}
                  <span class="col-start-2 col-end-4 truncate text-note text-text-faint">{ranOn(stretch)}</span>
                {/if}
                {#if tags.some((tag) => tag !== PIN)}
                  <span class="col-start-2 col-end-4 mt-tight flex flex-wrap gap-tight">
                    {#each tags.filter((tag) => tag !== PIN) as tag (tag)}
                      <span class="rounded-pill border border-edge-panel px-tight text-note text-text-faint">{tag}</span>
                    {/each}
                  </span>
                {/if}
                {#if stretch.current && last !== undefined}
                  <ContextBar room={stretch.room} run={last.run} />
                {/if}
              {/if}
            </a>
            <div class={["shrink-0 pt-tight pr-tight", inMain ? "" : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"]}>
              <SessionMenu
                named={named(stretch)}
                label={fill(say($lang, "world_row_name"), { room: titleOf(stretch), when: ago($lang, stretch.line.at, $tick) })}
                {tags}
                {pinning}
                session={{ name: stretch.line.name ?? "", run: stretch.current ? (last?.run ?? null) : null }}
              />
            </div>
          </li>
        {/each}
      </ul>
    {/each}
  </div>
</section>
