<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Proposal cards, the decide card's third kind (client/Spec.lean §4-55), on
  // a made-up city: a run in `shop/east` offered two cards on one plan,
  // one on the version the plan holds and one on a version it has since
  // left. Each card alone, one opened for editing, both above the
  // document in RefRain, the mailbox's deciding section holding them,
  // the letter one of them opens on the right side (roadmap A25), and
  // the mailbox key counting them.
  import { Address, B3Hash, RunId, Seq, TimeMs } from "../../wire";
  import type { Answer, DocumentState, EventRecord, ProposalCard, ProposalsAnswer, Query, Slice } from "../../wire";

  const SHOP = Address.make("shop");
  const PATH = "notes/plan.md";
  const DOC = Address.make(`shop/${PATH}`);
  const EAST = Address.make("shop/east");
  const RUN = RunId.make("0199c0de-0000-4c3d-8e4f-0000000000a1");

  const V0 = B3Hash.make("07c41e09".repeat(8));
  const V1 = B3Hash.make("3c1f0a9e".repeat(8));

  const HEAD = "# Plan\n\n";
  const BEFORE = "The reader opens a version. It edits in place. It saves the bytes.";
  const TEXT = `${HEAD}${BEFORE}\n\nDrafts are kept for a while.\n`;

  function bytes(text: string): number {
    return new TextEncoder().encode(text).length;
  }

  function slice(kind: Slice["kind"], text: string, trail: string): Slice {
    return { kind, lead: "", text, trail };
  }

  // The card on the version the plan holds: one sentence kept, two
  // rewritten.
  const CURRENT: ProposalCard = {
    id: B3Hash.make("a1".repeat(32)),
    run: RUN,
    baseline: V1,
    span: { start: bytes(HEAD), end: bytes(HEAD) + bytes(BEFORE) },
    slices: [
      slice("same", "The reader opens a version.", " "),
      slice("delete", "It edits in place.", " "),
      slice("insert", "It edits in the browser and keeps a draft per version.", " "),
      slice("delete", "It saves the bytes.", ""),
      slice("insert", "It saves the bytes of the version it was made on.", ""),
    ],
  };

  // The card on a version the plan has left.
  const STALE: ProposalCard = {
    id: B3Hash.make("b2".repeat(32)),
    run: RUN,
    baseline: V0,
    span: { start: 77, end: 105 },
    slices: [slice("delete", "Drafts are kept for a while.", ""), slice("insert", "Drafts are kept until the save lands.", "")],
  };

  const PROPOSALS: ProposalsAnswer = { doc: DOC, version: V1, open: [CURRENT, STALE] };

  const DOCUMENT: DocumentState = {
    held: {
      version: V1,
      format: "markdown",
      bytes: bytes(TEXT),
      body: { text: { encoding: "utf8", coverage: "whole", head: { span: { start: 0, end: bytes(TEXT) }, text: TEXT } } },
    },
  };

  const NOW = Date.now();

  function record(seq: number, kind: EventRecord["kind"], data: Record<string, unknown>): EventRecord {
    return {
      addr: EAST,
      data,
      kind,
      prev: B3Hash.make("0".repeat(64)),
      run: RUN,
      seq: Seq.make(seq),
      t: TimeMs.make(NOW - (10 - seq) * 60_000),
      v: 1,
      who: EAST,
    };
  }

  // The run, and the two offers this page folded, so the mailbox knows
  // the plan is worth asking about.
  const RECORDS: readonly EventRecord[] = [
    record(1, "run_started", { task: "tighten the plan's wording" }),
    record(2, "proposal_offered", { doc: DOC, baseline: V0, start: 77, end: 105, before: "", after: "" }),
    record(3, "proposal_offered", { doc: DOC, baseline: V1, start: 8, end: 75, before: "", after: "" }),
  ];

  function answering(query: Query): Answer | undefined {
    if (typeof query !== "object") return undefined;
    if ("proposals" in query) return query.proposals === DOC ? { proposals: PROPOSALS } : undefined;
    if ("document" in query) return query.document.at === DOC ? { document: { at: DOC, state: DOCUMENT } } : undefined;
    if ("sessions" in query) return { sessions: { room: query.sessions.room, sessions: [], earlier: 0 } };
    return undefined;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Column from "../mailbox/column.svelte";
  import Mailbox from "../mailbox/mailbox.svelte";
  import Letter from "../inspect/letter.svelte";
  import Card from "../refrain/proposals_card.svelte";
  import RefRain from "../refrain/refrain.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  const { lang } = ui();
  const LIVE = { kind: "live", city: "sprawling" } as const;
  const ignore = (): void => undefined;

  // The edited case is the card after a person pressed its edit answer;
  // the fixture presses it once, the way they would.
  let editedHost = $state<HTMLElement | undefined>(undefined);
  $effect(() => {
    const host = editedHost;
    if (host === undefined) return;
    const edit = say($lang, "proposal_edit");
    const button = [...host.querySelectorAll("button")].find((each) => each.textContent.trim() === edit);
    button?.click();
  });
</script>

<Stand link={LIVE} unread={[]} waiting={[]} answers={answering} records={RECORDS}>
  <Case label="proposal card · a change to accept, edit or reject">
    <Card doc={DOC} card={CURRENT} version={V1} />
  </Case>
  <Case label="proposal card · made on a version the document has left">
    <Card doc={DOC} card={STALE} version={V1} />
  </Case>
  <Case label="proposal card · edited before it is accepted">
    <div bind:this={editedHost}><Card doc={DOC} card={CURRENT} version={V1} /></div>
  </Case>
  <Case label="proposal cards · above the document in RefRain" width={600}>
    <div class="flex h-[640px] flex-col overflow-hidden bg-page">
      <RefRain building={SHOP} path={PATH} version={null} />
    </div>
  </Case>
  <Case label="mailbox · proposal cards in the deciding section" width={440}>
    <div class="flex h-[960px] flex-col bg-raised"><Column onClose={ignore} /></div>
  </Case>
  <Case label="letter · a proposal card opened on the right side, read as its diff" width={600}>
    <div class="flex h-[640px] flex-col overflow-hidden bg-page">
      <Letter doc={DOC} card={CURRENT.id} />
    </div>
  </Case>
  <Case label="mailbox key · two proposal cards waiting">
    <Mailbox asked={0} hint={(words: string) => words} />
  </Case>
</Stand>
