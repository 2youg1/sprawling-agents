<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The states of the Mayor room's box that only show once a person
  // acts: each of the four pills with its menu open, the card a refused
  // dispatch leaves where the reply would have been, and the line under
  // the box opened onto the four segments the run was told.
  //
  // Each menu opens upward from the foot of its case, as it does over
  // the box at the foot of the window, so a case reserves the height
  // of the longest menu above its trigger (`pt-menu` below); a menu
  // drawn outside its case is what `xtask render` refuses.
  import type { Answer, AxError, PrefixSegment, Query } from "../../wire";
  import { Address, B3Hash, RunId } from "../../wire";

  const HERE = Address.make("hall/mayor");
  const RUN = RunId.make("0199c0de-0000-4000-9000-00000000c0de");

  const NO_MODEL: AxError = {
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    action: "choose the main model",
    code: "E_MODEL_UNCHOSEN",
    nearby: [],
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "attach a provider on the settings page and pick a model for this tag",
    retry: "no",
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    subject: "no model is chosen for this tag",
  };

  const SLOTS: readonly PrefixSegment["slot"][] = ["city", "building", "resident", "run"];

  function segment(slot: PrefixSegment["slot"], text: string, at: string): PrefixSegment {
    return {
      bytes: text.length,
      // One hash per segment: the list is keyed by it.
      hash: B3Hash.make(String(SLOTS.indexOf(slot) + 1).repeat(64)),
      slot,
      sources: [{ addr: Address.make(at), dropped: 0, kept: text.length }],
      stored: true,
      text,
    };
  }

  // What the run was told, one segment per rung, as a city writes them.
  const TOLD: readonly PrefixSegment[] = [
    segment("city", "# CITY.md\nOne city, one ledger. Every resident answers to the rules of its building.", "CITY.md"),
    segment("building", "# hall\nThe hall plans the city's work and hands it to the buildings.", "hall/BUILDING.md"),
    segment("resident", "# mayor\nYou are the Mayor. Plan, dispatch, and say what the city does next.", "hall/mayor/URBANITE.md"),
    segment("run", "Answer the person in the words they wrote to you.", "hall/mayor"),
  ];

  function told(query: Query): Answer | undefined {
    return typeof query === "object" && "prefix" in query ? { prefix: { run: RUN, segments: TOLD } } : undefined;
  }
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { EFFORTS } from "../../core/commands";
  import { ui } from "../../ui";
  import { pills } from "../talk/composer";
  import PillView from "../talk/pill.svelte";
  import Failed from "../talk/failed.svelte";
  import ContextStrip from "../talk/context_strip.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();

  const ignore = (): void => undefined;
  const specs = $derived(
    pills(
      $lang,
      {
        served: MODELS.map((each) => ({ endpoint: "gallery", model: each.id, label: "gallery" })),
        chosen: { endpoint: "gallery", model: CHOSEN.id },
        session: null,
        rooms: ["hall/mayor", "atlas/api", "atlas/web"],
        here: HERE,
        effort: EFFORTS[2] ?? null,
        mode: "chat",
      },
      { model: ignore, workspace: ignore, effort: ignore, mode: ignore },
    ),
  );
  const NAMES = ["model", "workspace", "effort", "mode"] as const;
</script>

{#each NAMES as name, index (name)}
  {@const spec = specs[index]}
  {#if spec !== undefined}
    <Case label={`composer pill · the ${name} menu open`}>
      <div class="flex min-h-[36rem] items-end">
        <PillView {spec} starts="open" />
      </div>
    </Case>
  {/if}
{/each}

<Case label="conversation · a dispatch refused for want of a model">
  <Failed what={say($lang, "talk_failed_model")} error={NO_MODEL} onRetry={ignore} />
</Case>

<Case label="context strip · open on the four segments the run was told">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} answers={told}>
    <ContextStrip address={HERE} run={RUN} starts="open" />
  </Stand>
</Case>
