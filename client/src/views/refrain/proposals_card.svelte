<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One proposal card, the decide card's third kind (client/Spec.lean §7C,
  // 4-55): its body is the city's sentence-by-sentence diff, and its
  // answers are accept (y), edit then accept (e) and reject (n). The same
  // card stands in the mailbox and above the document in RefRain; the
  // seat hands in a line of its own (`lead`) and nothing else differs.
  //
  // **The card going away is the receipt.** A decision the city lands
  // closes the card, the answer for the document is asked again, and
  // this card is no longer drawn. Until then the card says where the
  // decision stands, and a refusal the city returns is written on it.
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";
  import { get } from "svelte/store";

  import { decideProposals } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, AxError, B3Hash, Command, ProposalCard, Slice } from "../../wire";
  import Decide from "../parts/decide.svelte";
  import type { Choice } from "../parts/decide.svelte";
  import { advance, editOf, refusesCard, rejectionOf, retaken, standingOf, takesAny, verdictsOf } from "./proposals";
  import type { Deciding, Edit, Happened, Take } from "./proposals";
  import { short } from "./reading";

  interface Props {
    // The document the card is on.
    readonly doc: Address;
    readonly card: ProposalCard;
    // The version the document holds now, from the same answer as the
    // card; `null` when it holds none.
    readonly version: B3Hash | null;
    // A line of the seat's own above the diff: in the mailbox the path
    // and the way to the document, in RefRain the way to the stretch.
    readonly lead?: Snippet;
  }

  const { doc, card, version, lead }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  let deciding = $state<Deciding>({ kind: "idle" });
  // The person's edit, kept while they go back to the diff and return.
  let edit = $state<Edit | null>(null);
  let editing = $state(false);

  const standing = $derived(standingOf(card, version));
  const busy = $derived(deciding.kind === "sent" || deciding.kind === "pending");
  const asker = $derived(
    fill(say($lang, "proposal_from"), { who: $belief.runs[card.run]?.addr ?? card.run.slice(0, 8) }),
  );

  function happen(happened: Happened): void {
    deciding = advance(deciding, happened);
  }

  function send(command: Command): void {
    if (!u.send(command)) return;
    happen({ kind: "sent", command });
  }

  function accept(): void {
    send(decideProposals(doc, [{ proposal: card.id, verdicts: verdictsOf(card, editing ? edit : null) }]));
  }

  function reject(): void {
    send(decideProposals(doc, [rejectionOf(card)]));
  }

  function toggleEditing(): void {
    edit ??= editOf(card);
    editing = !editing;
  }

  function retake(place: number, change: Partial<Take>): void {
    edit = retaken(edit ?? editOf(card), place, change);
  }

  // Why an accepting answer cannot be given now, if it cannot.
  const acceptWhy = $derived.by(() => {
    if (busy) return say($lang, "proposal_busy_why");
    if (standing.kind === "stale") return say($lang, "proposal_stale_why");
    if (editing && edit !== null && !takesAny(edit)) return say($lang, "proposal_nothing_taken");
    return undefined;
  });

  const choices = $derived<Choice[]>([
    {
      answer: "yes",
      label: say($lang, editing ? "proposal_accept_edited" : "proposal_accept"),
      why: acceptWhy,
      onPress: accept,
    },
    {
      answer: "edit",
      label: say($lang, editing ? "proposal_back" : "proposal_edit"),
      why: busy ? say($lang, "proposal_busy_why") : standing.kind === "stale" ? say($lang, "proposal_stale_why") : undefined,
      onPress: toggleEditing,
    },
    {
      answer: "no",
      label: say($lang, "proposal_reject"),
      why: busy ? say($lang, "proposal_busy_why") : undefined,
      onPress: reject,
    },
  ]);

  // The city's refusal of this card's decision, written on the card. A
  // version conflict means the document moved under the card: the
  // answer is asked again, and the card comes back stale.
  $effect(() => {
    let last: AxError | null = get(belief).refusal;
    return belief.subscribe((held) => {
      const error = held.refusal;
      if (error === null || error === last) return;
      last = error;
      untrack(() => {
        if (!refusesCard(error, doc, card.id)) return;
        happen({ kind: "refusal", error });
        if (error.code === "E_VERSION_CONFLICT") u.conn.asking.refresh({ proposals: doc });
      });
    });
  });

  // A decision in flight when the link drops is sent again, key and
  // all, when the link is back.
  $effect(() => {
    let live = get(u.conn.state).kind === "live";
    return u.conn.state.subscribe((state) => {
      const now = state.kind === "live";
      if (now === live) return;
      live = now;
      untrack(() => {
        if (!now) {
          happen({ kind: "lost" });
          return;
        }
        if (deciding.kind === "pending" && u.conn.command(deciding.command)) happen({ kind: "relinked" });
      });
    });
  });

  // Whether the sentence at `place` meets a struck one with no space
  // between them; the diff then sets the two half a character apart, so
  // the removed and the added words do not read as one.
  function abuts(place: number): boolean {
    const before = card.slices[place - 1];
    const here = card.slices[place];
    return before?.kind === "delete" && before.trail === "" && here?.lead === "";
  }

  // The first words of a sentence, for the name of its take box.
  function opening(slice: Slice): string {
    const words = slice.text.trim().split(/\s+/u).slice(0, 6).join(" ");
    return words.length < slice.text.trim().length ? `${words}…` : words;
  }
</script>

<Decide
  kind="proposal"
  {asker}
  at={fill(say($lang, "proposal_on"), { version: short(card.baseline) })}
  {choices}
>
  {#snippet body()}
    <div class="flex min-w-0 flex-col gap-snug">
      {@render lead?.()}
      {#if standing.kind === "stale"}
        <p class="text-note text-alert">
          {fill(say($lang, "proposal_stale"), {
            was: short(card.baseline),
            now: standing.now === null ? say($lang, "proposal_no_version") : short(standing.now),
          })}
        </p>
      {/if}
      <!-- Where a sent decision stands; the region is there before it has
      anything to say, so the first word it says is announced. -->
      <p class="text-note empty:hidden" role="status">{#if deciding.kind === "sent"}<span class="text-text-faint"
            >{say($lang, "proposal_deciding")}</span
          >{:else if deciding.kind === "pending"}<span class="text-text-faint">{say($lang, "proposal_pending")}</span
          >{:else if deciding.kind === "refused"}<span class="text-alert">{deciding.error.recovery}</span>{/if}</p>
      {#if editing && edit !== null}
        <ol class="proposal-text flex flex-col gap-snug">
          {#each card.slices as slice, place (place)}
            {@const take = edit.get(place)}
            {#if slice.kind === "same" || take === undefined}
              <li class="pl-[calc(var(--spacing-glyph-sm)+var(--spacing-snug)+var(--spacing-snug)+1px)] text-text-faint">
                {slice.text}
              </li>
            {:else}
              <li class="flex min-w-0 items-start gap-snug">
                <!-- The box stands centred on the first line of its row: a
                one-line textarea's height, which a struck row matches. -->
                <span class="flex h-[calc(1lh+2*var(--spacing-tight)+2px)] shrink-0 items-center">
                <input
                  type="checkbox"
                  class="size-glyph-sm accent-accent"
                  checked={take.taken}
                  aria-label={fill(say($lang, "proposal_take"), { words: opening(slice) })}
                  onchange={(event) => {
                    retake(place, { taken: event.currentTarget.checked });
                  }}
                />
                </span>
                {#if slice.kind === "delete"}
                  <del
                    class={[
                      "min-w-0 flex-1 rounded-control border border-transparent bg-alert/12 px-snug py-tight text-text decoration-alert",
                      !take.taken && "opacity-60",
                    ]}
                    >{slice.text}</del
                  >
                {:else}
                  <textarea
                    class={[
                      "field-sizing-content min-w-0 flex-1 resize-none rounded-control border border-edge-input bg-accent/12 px-snug py-tight text-body text-text",
                      !take.taken && "opacity-60",
                    ]}
                    rows="1"
                    aria-label={say($lang, "proposal_amend")}
                    value={take.text}
                    oninput={(event) => {
                      retake(place, { text: event.currentTarget.value });
                    }}
                  ></textarea>
                {/if}
              </li>
            {/if}
          {/each}
        </ol>
      {:else}
        <!-- One run of text, the way the city will land it: what stays,
        what goes (struck, on the alert's wash) and what comes (on the
        accent's), with each sentence's own spacing kept. -->
        <p class="proposal-text max-h-[16lh] overflow-y-auto whitespace-pre-wrap wrap-anywhere">
          {#each card.slices as slice, place (place)}{#if slice.kind === "same"}<span class="text-text-quiet"
                >{slice.lead}{slice.text}{slice.trail}</span
              >{:else if slice.kind === "delete"}{slice.lead}<del class="bg-alert/12 text-text decoration-alert"
                ><span class="sr-only">{say($lang, "proposal_removed")}</span>{slice.text}</del
              >{slice.trail}{:else}{slice.lead}<ins
                class={["bg-accent/12 text-text no-underline", abuts(place) && "ml-[0.5ch]"]}
                ><span class="sr-only">{say($lang, "proposal_added")}</span>{slice.text}</ins
              >{slice.trail}{/if}{/each}
        </p>
      {/if}
    </div>
  {/snippet}
</Decide>
