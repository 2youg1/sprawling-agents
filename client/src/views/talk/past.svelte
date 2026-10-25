<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // An earlier session of a room, in main (client/Spec.lean §7K): its runs as
  // the conversation draws them, and no box under them, because the city
  // goes on only with a room's current session. In the box's place stands
  // the way back to the room's current session. Going on from an earlier
  // session is a branch from one of its entries: each turn's own fork key
  // here, or the mailbox's fork entry, which branches from the session's
  // end. A branch brings main to the room's current session, which the
  // branch has become, once the city has opened it (`branching.ts`,
  // client D100): until then the line under the rule says it is
  // opening, and a refusal stays there with `/stop` beside it.
  //
  // In the panorama tier's band only the way back stands; the session
  // pane above it shows the session.
  import { newestWorking } from "../../core/belief/live";
  import { cancel } from "../../core/commands";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import type { Stretch } from "../../core/stretches";
  import { ui } from "../../ui";
  import type { Address, Seq } from "../../wire";
  import Button from "../parts/button.svelte";
  import AddressLine from "./address_line.svelte";
  import AsideLink from "./aside_link.look.svelte";
  import { branchInto } from "./branching";
  import { noteOf } from "./branch_note";
  import Scroller from "./scroller.svelte";
  import type { ForkPlan } from "./forking";
  import { drawsAddress, factsAt } from "./seat";
  import type { Seat } from "./seat";
  import Thread from "./thread.svelte";

  interface Props {
    readonly address: Address;
    // The session the address bar names, which the address line draws
    // even while `stretch` is null (4-10a: an earlier session draws its
    // own number, never the room's current one).
    readonly session: Seq;
    // That session as the city answered it; null while the city has not
    // answered for it, or no longer lists it.
    readonly stretch: Stretch | null;
    readonly band: boolean;
    // Where the column stands (`seat.ts`): whether it draws the address
    // line at its head (4-10a), and who states the frozen facts.
    readonly seat: Seat;
  }

  const { address, session, stretch, band, seat }: Props = $props();

  const u = ui();
  const { lang } = u;
  const { belief } = u.conn;
  const branching = u.branches.held;
  const note = $derived.by(() => {
    const here = branchInto($branching, address);
    const going = here.kind === "refused" ? newestWorking($belief, address) : undefined;
    const stop =
      going === undefined
        ? null
        : () => {
            u.send(cancel(going.run));
          };
    return noteOf(here, address, $lang, stop);
  });
</script>

<div class={["flex min-h-0 flex-col", band ? "" : "h-full"]}>
  {#if !band && drawsAddress(seat)}
    <AddressLine {address} {session} />
  {/if}
  {#if !band}
    <Scroller empty={false} rejoined={0} place={stretch === null ? address : `${address}:${String(stretch.line.began)}`}>
      {#each stretch?.runs ?? [] as run, at (run.run)}
        <Thread
          {run}
          facts={factsAt(seat, at)}
          onFork={(plan: ForkPlan) => {
            u.branches.open(address, { kind: "branch", origin: plan.origin }, () => {
              if (plan.draft !== null) u.prefs.setDraft(address, plan.draft);
            });
          }}
        />
      {/each}
    </Scroller>
  {/if}
  <div class="flex shrink-0 flex-col gap-snug border-t border-edge-panel pt-base">
    <p class="text-note text-text-quiet">{say($lang, "talk_past")}</p>
    <p class="text-note text-text-quiet">
      <AsideLink text={say($lang, "talk_current")} href={toFragment({ kind: "talk", address })} />
    </p>
    <div class="flex flex-wrap items-center gap-base">
      <p class="text-note text-text-quiet" aria-live="polite">{note?.said ?? ""}</p>
      {#if note !== null && note.stop !== null}
        <Button label={note.stop.label} name={note.stop.name} onPress={note.stop.onPress} />
      {/if}
    </div>
  </div>
</div>
