<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The run page's lenses besides time: the prompt as the city answers
  // it and as a person leaves it - one segment open with its copy
  // receipt showing, one the store no longer holds - then the context
  // lens over forty turns, and the evidence lens with and without
  // anything to check.

  import type { Answer, EvidenceItem, PrefixSegment, PrefixSlot, Query } from "../../wire";
  import { Address, B3Hash, Locator, RunId, Seq } from "../../wire";

  const RUN = RunId.make("5b1e7d2a-9c3f-4e8b-a1d2-3c4b5a6f7e80");

  function segmentOf(slot: PrefixSlot, at: number, stored: boolean, text: string): PrefixSegment {
    return {
      slot,
      bytes: text.length * 37,
      hash: B3Hash.make(String(at).repeat(64)),
      sources: [
        { addr: Address.make(`shop/${slot}/Memo.md`), kept: text.length, dropped: 0 },
        { addr: Address.make(`shop/${slot}/Plan.md`), kept: text.length, dropped: at === 2 ? 1_840 : 0 },
      ],
      stored,
      text,
    };
  }

  const SEGMENTS: readonly PrefixSegment[] = [
    segmentOf("city", 1, true, "You are a resident of the city sprawling. Keep to the building you were placed in."),
    segmentOf("building", 2, true, "The shop sells what its residents make.\nEvery change ships with a test."),
    segmentOf("resident", 3, false, ""),
    segmentOf("run", 4, true, "Tidy the release notes for 0.0.10."),
  ];

  const ITEMS: readonly EvidenceItem[] = [
    { at: Seq.make(412), kind: "screenshot", locator: Locator.make("shop/notes/.evidence/412.png"), picture: { width: 1440, height: 900, media_type: "image/png" } },
    { at: Seq.make(418), kind: "finished", locator: Locator.make("shop/notes/CHANGELOG.md"), picture: null },
  ];

  function answering(items: readonly EvidenceItem[]): (query: Query) => Answer | undefined {
    return (query) => {
      if (typeof query !== "object") return undefined;
      if ("prefix" in query) return { prefix: { run: query.prefix.run, segments: SEGMENTS } };
      if ("evidence" in query) return { evidence: { run: query.evidence.run, items } };
      return undefined;
    };
  }
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import Context from "../run/context.svelte";
  import Evidence from "../run/evidence.svelte";
  import { lookOf } from "../run/prompt";
  import PromptLook from "../run/prompt.look.svelte";
  import Prompt from "../run/prompt.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
  import { LIVE } from "./timed.svelte";

  const lang = ui().lang;
  // The first stored segment as a person leaves it after opening it and
  // pressing its copy key: the look drawn from the same decision the
  // seat makes, at the state a press puts it in.
  const OPENED = SEGMENTS[1]?.hash ?? "";
  const NOBODY = { toggle: () => undefined, copy: () => undefined, visit: () => undefined };
</script>

<Case label="run · the prompt lens as the city answers it">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(ITEMS)}>
    <Prompt run={RUN} />
  </Stand>
</Case>

<Case label="run · the prompt lens, one segment open with its copy receipt, one the store no longer holds">
  <PromptLook {...lookOf(SEGMENTS, { open: new Set([OPENED]), receipt: OPENED }, $lang, NOBODY)} />
</Case>

<Case label="run · the prompt lens of a run sent no prompt">
  <PromptLook {...lookOf([], { open: new Set(), receipt: null }, $lang, NOBODY)} />
</Case>

<Case label="run · the context lens over forty turns">
  <Context turns={LIVE} />
</Case>

{#each [ITEMS, []] as items, at (at)}
  <Case label={`run · the evidence lens ${items.length === 0 ? "with nothing to check" : "with a screenshot and a finish"}`}>
    <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={answering(items)}>
      <Evidence run={RUN} />
    </Stand>
  </Case>
{/each}
