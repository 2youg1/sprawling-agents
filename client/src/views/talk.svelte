<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The first page: one conversation with one room, the Mayor's unless
  // the address says otherwise. Every run in the room is a stretch of
  // the same thread; what waits for the person is a card in the same
  // thread; and the box at the bottom either steers the run that is
  // going or opens the next one.
  //
  // A session is a stretch of the room, not the room (roadmap S1): the
  // runs this session opened are the thread, everything before them
  // folds behind one line, and how the stretch began - freshly or by a
  // branch - is drawn where the two meet.
  //
  // Beside the conversation, once the room is wide enough to hold two
  // columns, stands what the run produced. The width that decides it is
  // the main region's, asked with a container query: an open rail takes
  // 232px of the window, so a layout that asked the window would put two
  // columns into a space that holds one.
  import { cancel, dispatch, openSession, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import type { RunBelief } from "../core/belief";
  import { fill, say } from "../core/lang";
  import { MAYOR, roomOf } from "../core/route";
  import { forkAsked } from "../core/forking";
  import type { Snippet } from "svelte";
  import type { Address, RoundsAnswer, Seq } from "../wire";
  import { ui } from "../ui";
  import Artifact, { PANEL_ID } from "./talk/artifact.svelte";
  import Composer from "./talk/composer.svelte";
  import Divider from "./talk/divider.svelte";
  import Forking from "./talk/forking.svelte";
  import type { Boundary, ForkPlan } from "./talk/forking";
  import Thread from "./talk/thread.svelte";
  import { anchorAt, footOf } from "./talk/anchoring";
  import type { Anchoring } from "./talk/anchoring";
  import { NOTHING, artifactsIn } from "./talk/trace";
  import Waiting from "./talk/waiting.svelte";

  interface Props {
    readonly address: Address;
  }

  const { address }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const effort = u.effort;
  const held = u.prefs.held;

  // The runs of this room, oldest first. A run whose room is not yet
  // known is not shown here rather than shown in the wrong room.
  const runs = $derived.by((): RunBelief[] =>
    Object.values($belief.runs)
      .filter((run) => run.addr === address)
      .sort((a, b) => (a.started ?? 0) - (b.started ?? 0) || a.lastSeq - b.lastSeq),
  );
  const live = $derived([...runs].reverse().find((run) => run.doing.kind !== "frozen"));
  const isMayor = $derived(address === MAYOR);
  const who = $derived(roomOf(address));

  // Where the newest session in this room began. Runs the fold places at
  // or before it are the earlier stretch, folded away (roadmap S1-3);
  // nothing before a boundary is ever a run of this stretch because a
  // session cannot open underneath a run working in the room.
  const began = $derived($belief.sessions[address] ?? null);
  const earlier = $derived(began === null ? [] : runs.filter((run) => run.lastSeq <= began));
  const shown = $derived(began === null ? runs : runs.filter((run) => run.lastSeq > began));

  // The run the panel speaks for: the one still going, or the last one
  // this room finished. The same question `Thread` asks, merged with it
  // by `asking` because the two ask it in the same words.
  const current = $derived(live ?? runs.at(-1));

  // The rounds of that run, asked here for the artifact panel and the
  // fork picker. Subscribed by hand because the run it follows changes:
  // `$store` binds one store at initialisation, and this question moves
  // to a new run the moment one starts.
  let answer = $state<RoundsAnswer | undefined>(undefined);
  $effect(() => {
    const run = current?.run;
    if (run === undefined) return;
    return u.conn.asking.ask({ rounds: { run } }).subscribe((held) => {
      answer = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  const artifacts = $derived(answer === undefined ? NOTHING : artifactsIn(answer.turns));
  // Whether there is a card to draw at all. Any one of the three panes
  // is enough; a run that read nothing, changed nothing and ran nothing
  // gets no card and no control to open one.
  const produced = $derived(
    artifacts.read !== null || artifacts.wrote !== null || artifacts.terminal !== null,
  );
  // Open until the person closes it, and forgotten on reload: this
  // belongs in the person's own `[ui]` section and there is no door to
  // it yet (client-SPEC 4-27).
  const panel = $derived($held.panel);

  // How this stretch began, in the words the divider draws. A branch
  // that went out from this screen knows its turn and its mother the
  // moment it is sent; a session somebody opened elsewhere is learned
  // from the record landing, and after a reload only the boundary's
  // existence survives - which is the part every case shares.
  let story = $state<Boundary | null>(null);
  // The next boundary change is ours when a branch just went out from
  // here: its record lands a round trip later, and that arrival must not
  // draw a second divider for somebody else's `/new`.
  let ours = false;
  let watching = false;
  let seen: Seq | null = null;

  $effect(() => {
    const opened = $belief.sessions[address] ?? null;
    if (!watching) {
      watching = true;
      seen = opened;
      return;
    }
    if (opened === null || opened === seen) return;
    seen = opened;
    if (ours) {
      ours = false;
      return;
    }
    story = { kind: "opened", at: u.now() };
  });

  function send(text: string): boolean {
    const going = live;
    if (going !== undefined) {
      return u.send(steer(going.run, text));
    }
    return u.send(
      dispatch({ addr: address, task: text, goal: say($lang, "talk_goal"), effort: $effort }),
    );
  }

  // One branch, from wherever a person pointed: the words a message
  // carries go back to the box through the draft door - the composer
  // reads that door when it mounts and this page remounts it below - and
  // the line it cut at travels in the one frame `/fork` sends (roadmap
  // S2-3).
  function doFork(plan: ForkPlan): void {
    if (!u.send(openSession(address, "nothing", plan.origin))) return;
    if (plan.draft !== null) u.prefs.setDraft(address, plan.draft);
    ours = true;
    story = { kind: "forked", at: u.now(), turn: plan.turn, mother: plan.mother };
    picking = false;
  }

  // `/fork` with no argument cannot name a line from inside the verb
  // table, so it asks this screen for the picker (`core/slash`'s one
  // outward request).
  let picking = $state(false);
  let asked = 0;
  $effect(() => {
    const asks = $forkAsked;
    if (asks !== asked) {
      asked = asks;
      picking = true;
    }
  });

  // Keep the newest words in view while the person is at the foot, and
  // leave them alone once they have scrolled up to read. The column
  // grows when answers land as well as when records do, so its size is
  // what is watched; the judgement itself is `talk/anchoring`'s (ux B1).
  let scroller = $state<HTMLDivElement | undefined>(undefined);
  let column = $state<HTMLDivElement | undefined>(undefined);
  let anchoring = $state<Anchoring>("follow");
  $effect(() => {
    const held = column;
    const box = scroller;
    if (held === undefined || box === undefined) return;
    // A room opens at its newest words; only the person's own scroll
    // moves it to reading back.
    anchoring = anchorAt({ kind: "opened" });
    box.scrollTop = box.scrollHeight;
    const watcher = new ResizeObserver(() => {
      const now = scroller;
      if (anchoring === "follow" && now !== undefined) now.scrollTop = now.scrollHeight;
    });
    watcher.observe(held);
    return () => {
      watcher.disconnect();
    };
  });

  // One composer, drawn in the middle of an empty room and in the bar
  // once the room has a thread. Two call sites, one instance at a time:
  // a second Composer would carry a second draft and a second selection.
  // It remounts when this stretch begins - which is how the words a
  // fork returned to the box arrive there, since the box reads its
  // draft once, when it mounts.
  const placeholder = $derived(
    isMayor
      ? say($lang, "talk_placeholder_mayor")
      : fill(say($lang, "talk_placeholder_room"), { room: who }),
  );

  // The checker types a `{#snippet}` name as a void call, which the
  // lint lane rejects inside a render tag. The name is taken again as
  // its `Snippet` type, and the template renders that.
  const composer: Snippet = drawComposer;
</script>

{#snippet drawComposer()}
  <Composer
    {placeholder}
    sending={sendingInto(live?.doing)}
    draft={address}
    hearing={u.hearing()}
    onSend={send}
    onStop={() => {
      const going = live;
      return going === undefined ? false : u.send(cancel(going.run));
    }}
  />
{/snippet}

<div class="flex min-h-0 flex-1 flex-col @lg/page:flex-row">
  <div class="flex min-h-0 flex-1 flex-col">
    <!-- The one control the panel has. It is here rather than on the
         panel because the panel is what it opens: a second control
         inside would be a second place to look for the same state. -->
    {#if produced}
      <div class="flex justify-end px-pane pt-snug">
        <button
          type="button"
          class="rounded-control px-snug py-tight text-note text-text-faint hover:bg-chrome hover:text-text-quiet"
          aria-expanded={panel}
          aria-controls={PANEL_ID}
          onclick={() => {
            u.prefs.setPanel(!panel);
          }}
        >
          {panel ? say($lang, "talk_panel_hide") : say($lang, "talk_panel_show")}
        </button>
      </div>
    {/if}
    <div
      bind:this={scroller}
      class="min-h-0 flex-1 overflow-y-auto"
      onscroll={(event) => {
        anchoring = anchorAt({ kind: "scrolled", foot: footOf(event.currentTarget) });
      }}
    >
      <!-- An empty room opens with the box about a third of the way down
           the page, because the first thing asked of a person here is to
           say something and a composer pinned to the foot of two
           thousand pixels of nothing reads as broken (ux A4). It rides
           down to the bar on the first send. -->
      <div
        bind:this={column}
        class={[
          "mx-auto flex min-h-full w-full max-w-talk flex-col px-pane pb-wide",
          runs.length === 0 ? "justify-start pt-[18vh]" : "justify-end pt-wide",
        ]}
      >
        <!-- The page's own name, and it is always here: a reader
             arriving by keyboard or by screen reader has something to
             land on, and `theme.css` hangs the view transition off
             `main h1`. Drawn as the heading of an empty room and read
             out but not drawn once the thread is what the page is
             about. -->
        <h1
          tabindex="-1"
          class={runs.length === 0 ? "sr-only" : "mb-wide text-note text-text-faint"}
        >
          {isMayor ? say($lang, "talk_empty_mayor") : who}
        </h1>
        {#if !isMayor && runs.length === 0}
          <p class="mb-wide text-note text-text-faint">{address}</p>
        {/if}
        {#if runs.length === 0}
          <div class="flex flex-col items-center gap-base py-section text-center">
            <p class="text-heading font-heading text-text-disabled">
              {isMayor
                ? say($lang, "talk_empty_mayor")
                : fill(say($lang, "talk_empty_room"), { room: who })}
            </p>
            <p class="text-note text-text-faint">
              {isMayor
                ? say($lang, "talk_opening_mayor")
                : fill(say($lang, "talk_opening_room"), { room: who })}
            </p>
            <div class="w-full">{@render composer()}</div>
          </div>
        {:else}
          <Divider {earlier} {who} boundary={story} onFork={doFork} onRetry={send} />
          {#each shown as run (run.run)}
            <Thread {run} {who} onFork={doFork} onRetry={send} />
          {/each}
          <Waiting />
        {/if}
      </div>
    </div>
    {#if runs.length > 0}
      <div class="relative mx-auto w-full max-w-talk px-pane pb-pane">
        {#key story}
          {@render composer()}
        {/key}
        {#if picking && answer !== undefined}
          <Forking
            rounds={answer}
            onFork={doFork}
            onClose={() => {
              picking = false;
            }}
          />
        {/if}
      </div>
    {/if}
  </div>
  {#if produced}
    <Artifact artifacts={artifacts} open={panel} />
  {/if}
</div>
