

<script lang="ts">
  
  
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
  import { draftPlace } from "./draft";

  interface Props {
    readonly address: Address;
    
    
    readonly session: Seq;
    
    
    readonly stretch: Stretch | null;
    readonly band: boolean;
    
    
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
    <Scroller
      empty={false}
      rejoined={0}
      place={stretch === null ? address : `${address}:${String(stretch.line.began)}`}
    >
      {#each stretch?.runs ?? [] as run, at (run.run)}
        <Thread
          {run}
          facts={factsAt(seat, at, run.run)}
          onFork={(plan: ForkPlan) => {
            u.branches.open(address, { kind: "branch", origin: plan.origin }, () => {
              if (plan.draft !== null) {
                u.prefs.setDraft(
                  draftPlace(address, $belief.sessions[address] ?? null),
                  plan.draft,
                );
              }
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
