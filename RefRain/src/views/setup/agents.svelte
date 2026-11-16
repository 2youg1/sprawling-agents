<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { addAgent } from "../../core/access";
  import { ui } from "../../ui";
  import type { AgentOffer } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import { panelBeneath } from "../settings/hosted.svelte";
  import { readingOf } from "./agents";
  import Look from "./agents_page.svelte";

  const u = ui();
  const asked = u.conn.asking.ask(QUERIES.agentCatalog);
  const read = $derived(
    readAnswer($asked, (held) => ("agent_catalog" in held ? held.agent_catalog : undefined)),
  );

  let text = $state("");
  let adding = $state<ReadonlySet<string>>(new Set());
  const reading = $derived(readingOf(text, read.kind === "held" ? read.value : undefined));
  
  
  let parsed = $state<AgentOffer | null>(null);
  $effect(() => {
    if (reading.kind !== "paste") {
      parsed = null;
      return;
    }
    return u.conn.asking.ask({ parse_agent_spec: { text: reading.text } }).subscribe((now) => {
      parsed = now !== undefined && "agent_spec" in now ? now.agent_spec : null;
    });
  });

  
  const room = $derived.by(() => {
    const under = panelBeneath();
    return under.kind === "talk" ? under.address : null;
  });

  
  $effect(() => {
    if (read.kind !== "held") return;
    const added = new Set(read.value.added.map((line) => line.id));
    if ([...adding].some((id) => added.has(id)))
      adding = new Set([...adding].filter((id) => !added.has(id)));
  });

  function add(offer: AgentOffer, here: boolean): void {
    if (!u.send(addAgent(offer, here ? room : null))) return;
    adding = new Set([...adding, offer.id]);
    text = "";
    u.conn.asking.refresh(QUERIES.agentCatalog);
  }

  
  function paste(): void {
    void navigator.clipboard
      .readText()
      .then((held) => (text = held))
      .catch(() => undefined);
  }
</script>

{#if read.kind === "held"}
  <Look
    answer={read.value}
    {text}
    {reading}
    {parsed}
    {room}
    {adding}
    onText={(next: string) => {
      text = next;
    }}
    onPaste={paste}
    onAdd={add}
  />
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.agentCatalog} />
{/if}
