

<script lang="ts">
  
  
  import { cancel, dispatch, steer } from "../core/commands";
  import { sendingInto } from "../core/doing";
  import { newestWorking } from "../core/belief/live";
  import { heldIn } from "../core/belief/rooms";
  import { fill, say } from "../core/lang";
  import { hhmmss } from "../core/time";
  import { landingOf, sentFrom } from "../core/landing";
  import type { Landing, Sent } from "../core/landing";
  import { MAYOR } from "../core/route";
  import { roundsThrough } from "./talk/forking";
  import { lineIn } from "../core/stretches";
  import { QUERIES } from "../core/asking";
  import { forkAsked } from "../core/forking";
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";
  import type { Address, RoundsAnswer, Seq } from "../wire";
  import { ui } from "../ui";
  import AddressLine from "./talk/address_line.svelte";
  import { drawsAddress, factsAt } from "./talk/seat";
  import type { Seat } from "./talk/seat";
  import Composer from "./talk/composer.svelte";
  import Divider from "./talk/divider.svelte";
  import Forking from "./talk/forking.svelte";
  import Button from "./parts/button.svelte";
  import { branchInto } from "./talk/branching";
  import { noteOf } from "./talk/branch_note";
  import Landed from "./talk/landed.svelte";
  import type { Boundary, ForkPlan } from "./talk/forking";
  import { draftPlace } from "./talk/draft";
  import Thread from "./talk/thread.svelte";
  import { called } from "./talk/naming";
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
    
    
    readonly band: boolean;
    
    
    readonly seat: Seat;
  }

  const { address, band, seat }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const effort = u.effort;
  const policy = u.policy;
  const held = u.prefs.held;

  
  const runs = $derived(heldIn($belief, address));
  const live = $derived(newestWorking($belief, address));
  
  
  const branching = u.branches.held;
  const branchNote = $derived.by(() => {
    const going = live;
    const stop =
      going === undefined
        ? null
        : () => {
            u.send(cancel(going.run));
          };
    return noteOf(branchInto($branching, address), address, $lang, stop);
  });
  const isMayor = $derived(address === MAYOR);
  
  
  const identity = u.conn.asking.ask(QUERIES.identity);
  const stated = $derived(
    $identity !== undefined && "identity" in $identity && "stated" in $identity.identity
      ? $identity.identity.stated.mayor
      : null,
  );
  const who = $derived(called(address, stated, $lang));

  
  const began = $derived($belief.sessions[address] ?? null);
  const sessionAnswer = $derived(u.conn.asking.ask({ sessions: { room: address } }));
  const currentLine = $derived(
    lineIn(
      $sessionAnswer !== undefined && "sessions" in $sessionAnswer
        ? $sessionAnswer.sessions
        : undefined,
      undefined,
    ),
  );
  const origin = $derived(
    currentLine?.began === began ? (currentLine.branched_from ?? null) : null,
  );
  const mother = $derived(origin === null ? undefined : $belief.runs[origin.run]);
  const earlier = $derived(
    origin !== null
      ? mother === undefined
        ? []
        : [mother]
      : began === null
        ? []
        : runs.filter((run) => run.lastSeq <= began),
  );
  const shown = $derived(began === null ? runs : runs.filter((run) => run.lastSeq > began));

  
  const changed = $derived.by(() => {
    const change = $belief.policies[address] ?? null;
    return change === null || (began !== null && change.seq <= began) ? null : change;
  });
  const policyAt = $derived(
    changed === null ? -1 : shown.findIndex((run) => (run.started ?? 0) > changed.at),
  );
  const policyLine = $derived(
    changed === null
      ? null
      : fill(say($lang, "talk_policy_changed"), {
          mode: say($lang, `mode_${changed.policy.mode}`),
          limit: say($lang, `admission_value_${changed.policy.write}`),
        }),
  );

  
  const current = $derived(
    live ?? shown.at(-1) ?? (origin === null ? undefined : $belief.runs[origin.run]),
  );

  
  let rawAnswer = $state<RoundsAnswer | undefined>(undefined);
  const answer = $derived(
    rawAnswer === undefined
      ? undefined
      : roundsThrough(
          rawAnswer,
          live === undefined && shown.length === 0 ? (origin?.at_seq ?? null) : null,
        ),
  );
  $effect(() => {
    const run = current?.run;
    rawAnswer = undefined;
    if (run === undefined) return;
    return u.conn.asking.ask({ rounds: { run } }).subscribe((held) => {
      rawAnswer = held !== undefined && "rounds" in held ? held.rounds : undefined;
    });
  });

  
  const said = $derived(answer?.turns.filter((turn) => typeof turn.said === "string").at(-1));

  
  let story = $state<Boundary | null>(null);
  
  
  let ours = false;
  let watching = false;
  let seen: Seq | null = null;
  const boundary = $derived.by((): Boundary | null => {
    if (origin !== null) {
      const local = story?.kind === "forked" && story.mother === origin.run ? story : null;
      return {
        kind: "forked",
        at: local?.at ?? null,
        turn: local?.turn ?? null,
        mother: origin.run,
        origin,
      };
    }
    return story;
  });

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

  
  let sent = $state<Sent | null>(null);
  const landing = $derived.by((): Landing => {
    if (sent?.from !== address) return { kind: "pending" };
    const found = landingOf(sent, $belief.runs);
    return found.kind === "elsewhere" && $belief.runs[found.run]?.doing.kind === "frozen"
      ? { kind: "pending" }
      : found;
  });

  
  const link = u.conn.state;
  function hear(): Heard {
    return { live: $link.kind === "live", newest: newestSeq($belief), refusal: $belief.refusal };
  }
  let delivery = $state<Delivery>(NONE);
  
  
  $effect(() => {
    const kind = delivery.kind;
    if (kind === "none") return;
    if (kind === "accepted") {
      if (sent === null || landing.kind !== "pending") delivery = NONE;
      return;
    }
    delivery = delivered(
      untrack(() => delivery),
      hear(),
    );
  });
  const echo = $derived(delivery.kind === "none" ? null : delivery);

  
  const blank = $derived(shown.length === 0 && origin === null && echo === null);
  
  
  const addressed = $derived(!band && drawsAddress(seat));

  function send(text: string): boolean {
    const going = live;
    if (going !== undefined) {
      const steered = u.send(steer(going.run, text));
      if (steered) {
        sent = null;
        delivery = leftTheBox(text, hear());
        awaitAnswer(text);
      }
      return steered;
    }
    const went = u.send(
      dispatch({
        addr: address,
        task: text,
        goal: say($lang, "talk_goal"),
        effort: $effort,
        policy: $policy,
      }),
    );
    if (went) {
      sent = sentFrom(address, text, $belief.runs);
      delivery = leftTheBox(text, hear());
      awaitAnswer(text);
    }
    return went;
  }

  
  function awaitAnswer(text: string): void {
    refused = null;
    handing = hand(text, $belief);
    u.conversing.set({ kind: "waiting", handing });
  }

  
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
  $effect(() => () => {
    u.conversing.set(NOT_CONVERSING);
  });

  
  function doFork(plan: ForkPlan): void {
    picking = false;
    u.branches.open(address, { kind: "branch", origin: plan.origin }, () => {
      if (plan.draft !== null) {
        u.prefs.setDraft(draftPlace(address, $belief.sessions[address] ?? null), plan.draft);
      }
      ours = true;
      story = {
        kind: "forked",
        at: u.now(),
        turn: plan.turn,
        mother: plan.mother,
        origin: plan.origin,
      };
      
      rejoined += 1;
    });
  }

  
  let picking = $state(false);
  let asked = 0;
  $effect(() => {
    const asks = $forkAsked;
    if (asks !== asked) {
      asked = asks;
      picking = true;
    }
  });

  
  let rejoined = $state(0);

  
  const placeholder = $derived(
    isMayor && stated === null
      ? say($lang, "talk_placeholder_mayor")
      : fill(say($lang, "talk_placeholder_room"), { room: who }),
  );

  
  const composer: Snippet = drawComposer;
</script>

{#snippet addressInBox()}
  <AddressLine {address} at="box" />
{/snippet}

{#snippet lastSaid()}
  {#if said !== undefined}
    <div class="mb-snug flex flex-col gap-tight">
      <div class="flex items-baseline gap-base text-note text-text-faint">
        <span class="font-label text-text"
          >{called(address, answer?.opening?.names?.mayor, $lang)}</span
        >
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
      what={refused.error.code === "E_MODEL_UNCHOSEN"
        ? say($lang, "talk_failed_model")
        : say($lang, "talk_failed_refused")}
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
    started={echo !== null}
    sending={sendingInto(live?.doing)}
    draft={draftPlace(address, began)}
    hearing={u.hearing()}
    room={address}
    band={band ? lastSaid : undefined}
    door={addressed ? addressInBox : undefined}
    onSend={send}
    onStop={() => {
      const going = live;
      return going === undefined ? false : u.send(cancel(going.run));
    }}
  />
{/snippet}


<div
  class={["flex min-h-0 flex-col", band ? "" : "h-full"]}
  style:container-type={band ? undefined : "size"}
>
  {#if !band}
    <Scroller empty={blank} {rejoined} place={address}>
      {#if shown.length > 0 || origin !== null}
        <div class="mb-base flex justify-end"><Showing /></div>
        {#if drawsCalls($held.showing)}
          <Divider
            session={began}
            {earlier}
            shown={shown.length}
            {boundary}
            onFork={doFork}
            onRetry={send}
          />
          {#each shown as run, at (run.run)}
            {#if policyLine !== null && policyAt === at}
              <p class="my-snug text-center text-note text-text-faint">{policyLine}</p>
            {/if}
            <Thread {run} facts={factsAt(seat, at, run.run)} onFork={doFork} onRetry={send} />
          {/each}
          {#if policyLine !== null && policyAt === -1}
            <p class="my-snug text-center text-note text-text-faint">{policyLine}</p>
          {/if}
        {:else}
          <Stream session={began} {shown} {earlier} {boundary} />
        {/if}
      {/if}
      {#if echo !== null}
        <Delivered delivery={echo} />
      {/if}
      
      <Waiting />
      <Inbox addr={address} />
    </Scroller>
  {/if}
  <div
    class="relative shrink-0 transition-transform duration-page"
    style:transform={!band && blank
      ? "translateY(calc(-50cqh + 50% + var(--spacing-margin)))"
      : undefined}
  >
    {#if branchNote !== null}
      <div class="mb-tight flex flex-wrap items-center gap-base">
        <p class="text-note text-text-faint" role="status">{branchNote.said}</p>
        {#if branchNote.stop !== null}
          <Button
            label={branchNote.stop.label}
            name={branchNote.stop.name}
            onPress={branchNote.stop.onPress}
          />
        {/if}
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
