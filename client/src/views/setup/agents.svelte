<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The ACP agents page (client/Spec.lean §4-67): asks the city for its
  // agent catalog - what this computer has, the bundled registry
  // snapshot, and what is already added - and, when the box holds a
  // pasted launch spec, for the city's reading of it; then hands both
  // to the page (`agents_page.svelte`), which the gallery draws from a
  // fixture. Adding is `AddAgent`, carrying the digest of the launch
  // spec the card showed, so the city refuses a card that changed under
  // the person's eyes; signing in is `AgentLogin`.
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { addAgent, agentLogin } from "../../core/commands";
  import { ui } from "../../ui";
  import type { AgentOffer } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import { panelBeneath } from "../settings/hosted.svelte";
  import { readingOf } from "./agents";
  import Look from "./agents_page.svelte";

  const u = ui();
  const asked = u.conn.asking.ask(QUERIES.agentCatalog);
  const read = $derived(readAnswer($asked, (held) => ("agent_catalog" in held ? held.agent_catalog : undefined)));

  let text = $state("");
  let adding = $state<ReadonlySet<string>>(new Set());
  const reading = $derived(readingOf(text, read.kind === "held" ? read.value : undefined));
  // Asked only for a paste, once per distinct text; a refusal lands
  // where every refusal does and leaves no card.
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

  // The room the settings panel stands over, which "add and use here" seats in.
  const room = $derived.by(() => {
    const under = panelBeneath();
    return under.kind === "talk" ? under.address : null;
  });

  // An offer stops being "being added" once the catalog lists it as added.
  $effect(() => {
    if (read.kind !== "held") return;
    const added = new Set(read.value.added.map((line) => line.id));
    if ([...adding].some((id) => added.has(id))) adding = new Set([...adding].filter((id) => !added.has(id)));
  });

  function add(offer: AgentOffer, here: boolean): void {
    if (!u.send(addAgent(offer, here ? room : null))) return;
    adding = new Set([...adding, offer.id]);
    text = "";
    u.conn.asking.refresh(QUERIES.agentCatalog);
  }

  function login(agent: string, method: string): void {
    if (u.send(agentLogin(agent, method))) u.conn.asking.refresh(QUERIES.agentCatalog);
  }

  // The paste key, for a person who cannot hold a chord: what the
  // clipboard holds, if the browser lets the page read it.
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
    onLogin={login}
  />
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.agentCatalog} />
{/if}
