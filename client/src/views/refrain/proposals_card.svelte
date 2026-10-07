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
  // This file holds the card's state and its effects; `proposals_card.ts`
  // turns them into the value `proposals_card.look.svelte` draws.
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
  import type { Address, AxError, B3Hash, Command, ProposalCard } from "../../wire";
  import Decide from "../parts/decide.svelte";
  import type { Choice } from "../parts/decide.svelte";
  import { advance, editOf, refusesCard, rejectionOf, retaken, standingOf, verdictsOf } from "./proposals";
  import type { Deciding, Edit, Happened, Take } from "./proposals";
  import { cardLookOf, refusalsOf } from "./proposals_card";
  import Look from "./proposals_card.look.svelte";
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
  // The card as its look and its refusals read it: the edit only while
  // the card is open for editing.
  const held = $derived({ standing, deciding, edit: editing ? edit : null, lead });
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
    send(decideProposals(doc, [{ proposal: card.id, verdicts: verdictsOf(card, held.edit) }]));
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

  const refusals = $derived(refusalsOf(held, $lang));

  const choices = $derived<Choice[]>([
    {
      answer: "yes",
      label: say($lang, editing ? "proposal_accept_edited" : "proposal_accept"),
      why: refusals.accept,
      onPress: accept,
    },
    {
      answer: "edit",
      label: say($lang, editing ? "proposal_back" : "proposal_edit"),
      why: refusals.edit,
      onPress: toggleEditing,
    },
    {
      answer: "no",
      label: say($lang, "proposal_reject"),
      why: refusals.reject,
      onPress: reject,
    },
  ]);

  const look = $derived(cardLookOf(card, held, $lang, { retake }));

  // The city's refusal of this card's decision, written on the card. A
  // version conflict means the document moved under the card: the
  // answer is asked again, and the card comes back stale.
  $effect(() => {
    let last: AxError | null = get(belief).refusal;
    return belief.subscribe((believed) => {
      const error = believed.refusal;
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
</script>

<Decide
  kind="proposal"
  {asker}
  at={fill(say($lang, "proposal_on"), { version: short(card.baseline) })}
  {choices}
>
  {#snippet body()}
    <Look {...look} />
  {/snippet}
</Decide>
