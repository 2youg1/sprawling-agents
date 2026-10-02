<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The conversation: one room, the Mayor's unless the address says
  // otherwise, as the column the shell's grid gives it (client/Spec.lean §4-33)
  // - words flowing down it, the box at its foot. Every run in the room
  // is a stretch of the same thread; what waits for the person is a card
  // in it; and the box either steers the run that is going or opens the
  // next one. In the panorama tier the column is a band along the bottom:
  // the last thing said, and the same box (client/Spec.lean §7I).
  //
  // A session is a stretch of the room, not the room (roadmap S1): runs
  // before it fold behind one line, and how it began is drawn where the
  // two meet.
  import { cancel, dispatch, openSession, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { fill, say } from "../core/lang";
  import { hhmmss } from "../core/time";
  import { landingOf, sentFrom } from "../core/landing";
  import type { Landing, Sent } from "../core/landing";
  import { MAYOR, roomOf } from "../core/route";
  import { forkAsked } from "../core/forking";
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";
  import type { Address, RoundsAnswer, Seq } from "../wire";
  import { ui } from "../ui";
  import Composer from "./talk/composer.svelte";
  import Divider from "./talk/divider.svelte";
  import Forking from "./talk/forking.svelte";
  import Landed from "./talk/landed.svelte";
  import type { Boundary, ForkPlan } from "./talk/forking";
  import Thread from "./talk/thread.svelte";
  import Showing from "./shared/showing.svelte";
  import Stream from "./talk/stream.svelte";
  import { drawsCalls } from "../core/results";
  import Scroller from "./talk/scroller.svelte";
  import Delivered from "./talk/delivered.svelte";
  import { NONE, delivered, sent as leftTheBox } from "./talk/delivery";
  import type { Delivery, Heard } from "./talk/delivery";
  import Inbox from "./talk/inbox.svelte";
  import Waiting from "./talk/waiting.svelte";
  import Failed from "./talk/failed.svelte";
  import { IDLE, NOT_CONVERSING, hand, newestSeq, settle } from "./talk/handing";
  import type { Handing } from "./talk/handing";
  import type { AxError } from "../wire";

  interface Props {
    readonly address: Address;
    // The panorama tier's band: the thread steps aside and the last thing
    // said stands above the box.
    readonly band: boolean;
  }

  const { address, band }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const effort = u.effort;
  const mode = u.mode;
  const held = u.prefs.held;

  // The runs of this room, oldest first. A run whose room is not yet
  // known is not shown here rather than shown in the wrong room.
  const runs = $derived(heldIn($belief, address));
  const live = $derived(newestWorking($belief, address));
  const isMayor = $derived(address === MAYOR);
  const who = $derived(roomOf(address));

  // Where the newest session in this room began. Runs the fold places at
  // or before it are the earlier stretch, folded away (roadmap S1-3);
  // nothing before a boundary is ever a run of this stretch because a
  // session cannot open underneath a run working in the room.
  const began = $derived($belief.sessions[address] ?? null);
  const earlier = $derived(began === null ? [] : runs.filter((run) => run.lastSeq <= began));
  const shown = $derived(began === null ? runs : runs.filter((run) => run.lastSeq > began));

  // The run the band and the fork picker speak for: the one still going,
  // or the last one this room finished. The same question `Thread` asks, merged with it
  // by `asking` because the two ask it in the same words.
  const current = $derived(live ?? runs.at(-1));

  // The rounds of that run, asked here for the band's last line and the
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

  // The last thing said in this room, which the panorama band shows above
  // the box: the newest run's reply as the rounds have it.
  const said = $derived(answer?.turns.filter((turn) => typeof turn.said === "string").at(-1));

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

  // The last dispatch sent from this room, and where its run started:
  // a bare building's work opens a room of its own (client D6).
  // The line saying so is about the dispatch, so a steer sent after it
  // or the run it started ending takes the line away.
  let sent = $state<Sent | null>(null);
  const landing = $derived.by((): Landing => {
    if (sent?.from !== address) return { kind: "pending" };
    const found = landingOf(sent, $belief.runs);
    return found.kind === "elsewhere" && $belief.runs[found.run]?.doing.kind === "frozen" ? { kind: "pending" } : found;
  });

  // Where the words last sent stand (`talk/delivery.ts`): read off the
  // link and the belief while there is something to settle, and drawn at
  // the foot of the thread until the run's own thread holds them.
  const link = u.conn.state;
  function hear(): Heard {
    return { live: $link.kind === "live", newest: newestSeq($belief), refusal: $belief.refusal };
  }
  let delivery = $state<Delivery>(NONE);
  // Accepted words leave the foot once the run they started is on screen
  // (or went to another room, which `Landed` says); a steer is heard by a
  // run already on screen.
  $effect(() => {
    const kind = delivery.kind;
    if (kind === "none") return;
    if (kind === "accepted") {
      if (sent === null || landing.kind !== "pending") delivery = NONE;
      return;
    }
    delivery = delivered(untrack(() => delivery), hear());
  });
  const echo = $derived(delivery.kind === "none" ? null : delivery);

  // An empty room: nothing to read and nothing just sent, so the box
  // stands in the middle. The first send lets it sink at once, before
  // the city has answered, because the words it shows are already there.
  const blank = $derived(runs.length === 0 && echo === null);

  function send(text: string): boolean {
    const going = live;
    if (going !== undefined) {
      const steered = u.send(steer(going.run, text));
      if (steered) {
        sent = null;
        delivery = leftTheBox(text, hear());
      }
      return steered;
    }
    const went = u.send(
      dispatch({ addr: address, task: text, goal: say($lang, "talk_goal"), effort: $effort, mode: $mode }),
    );
    if (went) {
      sent = sentFrom(address, text, $belief.runs);
      delivery = leftTheBox(text, hear());
      refused = null;
      handing = hand(text, $belief);
      u.conversing.set({ kind: "waiting", handing });
    }
    return went;
  }

  // A refused dispatch never becomes a run: its card stands where the reply would have been
  // (`handing.ts` pairs a refusal with its send), and the corner is told the card is ours.
  let handing: Handing = IDLE;
  let refused = $state<{ readonly words: string; readonly error: AxError } | null>(null);
  $effect(() => {
    const now = $belief;
    const held = handing;
    const next = settle(held, now, "");
    handing = next.handing;
    if (held.kind !== "handed" || next.handing.kind !== "idle") return;
    const error = next.box === null ? null : now.refusal;
    if (error !== null) refused = { words: held.words, error };
    u.conversing.set(error === null ? NOT_CONVERSING : { kind: "answered", refusal: error });
  });
  $effect(() => () => { u.conversing.set(NOT_CONVERSING); });

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
    // The branch is where the person just went.
    rejoined += 1;
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

  // Where the view stands while words arrive is the scroller's
  // (`talk/scroller.svelte`); this page only sends it to the foot when a
  // branch it made opens there.
  let rejoined = $state(0);

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

{#snippet lastSaid()}
  {#if said !== undefined}
    <div class="mb-snug flex flex-col gap-tight">
      <div class="flex items-baseline gap-base text-note text-text-faint">
        <span class="font-label text-text">{isMayor ? say($lang, "talk_empty_mayor") : who}</span>
        <span class="figure">{hhmmss(said.t)}</span>
      </div>
      <p class="truncate text-body text-text">{said.said}</p>
    </div>
  {/if}
{/snippet}

{#snippet drawComposer()}
  <Landed {landing} />
  {#if refused !== null}
    <Failed
      what={refused.error.code === "E_MODEL_UNCHOSEN" ? say($lang, "talk_failed_model") : say($lang, "talk_failed_refused")}
      error={refused.error}
      onRetry={() => {
        const words = refused?.words;
        refused = null;
        if (words !== undefined) send(words);
      }}
    />
  {/if}
  <Composer
    {placeholder}
    sending={sendingInto(live?.doing)}
    draft={address}
    hearing={u.hearing()}
    room={address}
    band={band ? lastSaid : undefined}
    onSend={send}
    onStop={() => {
      const going = live;
      return going === undefined ? false : u.send(cancel(going.run));
    }}
  />
{/snippet}

<!-- One column, one box. The box is the same element in an empty room and
in a full one: in an empty room it is lifted to the middle of the column
with the room's name above it, and the first send lets it sink to its seat
(client/Spec.lean §7I), so neither the words being typed nor an input method's
composition is rebuilt on the way. -->
<div class={["flex min-h-0 flex-col", band ? "" : "h-full"]} style:container-type={band ? undefined : "size"}>
  {#if !band}
    <Scroller empty={blank} {rejoined}>
      {#if runs.length > 0}
        <div class="mb-base flex justify-end"><Showing /></div>
        {#if drawsCalls($held.showing)}
          <Divider {earlier} {who} boundary={story} onFork={doFork} onRetry={send} />
          {#each shown as run, at (run.run)}
            <Thread {run} {who} opens={at === 0} onFork={doFork} onRetry={send} />
          {/each}
        {:else}
          <Stream {shown} {earlier} boundary={story} />
        {/if}
      {/if}
      {#if echo !== null}
        <Delivered delivery={echo} />
      {/if}
      <!-- The queue is drawn in an empty room too: "N waiting" links to
      the mayor's room, which a person who only worked in a building has
      never spoken in. -->
      <Waiting />
      <Inbox addr={address} />
    </Scroller>
  {/if}
  <div
    class="relative shrink-0 transition-transform duration-page"
    style:transform={!band && blank ? "translateY(calc(-50cqh + 50% + var(--spacing-margin)))" : undefined}
  >
    {#if !band && blank}
      <div class="absolute inset-x-0 bottom-full mb-wide text-center">
        <p class="text-title font-title text-text">{isMayor ? say($lang, "talk_empty_mayor") : who}</p>
        <p class="text-note text-text-faint">{address}</p>
      </div>
    {/if}
    {#if story?.kind === "forked" && shown.length === 0}
      <p class="mb-tight text-note text-text-faint" role="status">
        {fill(say($lang, "fork_pending"), { turn: String(story.turn) })}
      </p>
    {/if}
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
</div>
