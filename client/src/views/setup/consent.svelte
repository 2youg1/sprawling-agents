<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { AgentOffer } from "../../wire";

  // One agent's consent card (client/Spec.lean §4-67): the add key sits
  // on the card that shows exactly what will run, so pressing it is the
  // consent and no dialog follows. The card shows the launch line as the
  // city will run it, the version and whether it is pinned, where the
  // entry came from, its licence, the names of the environment variables
  // it sets, and that it runs with the person's own permissions.
  export interface ConsentProps {
    readonly offer: AgentOffer;
    // Whether the name heads the card; a row that already shows it does not.
    readonly named: boolean;
    // Whether there is a room to seat it in, which offers "add and use here".
    readonly seats: boolean;
    // The press went out and the catalog has not answered since.
    readonly adding: boolean;
    readonly onAdd: (offer: AgentOffer, here: boolean) => void;
  }
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import { PINNED, SOURCE, loginKey, programOf } from "./agents";

  const { offer, named, seats, adding, onAdd }: ConsentProps = $props();
  const { lang } = ui();
  const pinned = $derived(PINNED[offer.pinned]);
  const version = $derived(offer.version ?? say($lang, "acp_version_unknown"));
  const found = $derived(
    offer.source === "detected" ? ` · ${fill(say($lang, "acp_on_path"), { program: programOf(offer) })}` : "",
  );
</script>

<div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-base" role="group" aria-label={offer.name}>
  {#if named}
    <div class="flex flex-wrap items-baseline justify-between gap-x-base gap-y-tight">
      <span class="text-label font-label text-text">{offer.name}</span>
      <span class="text-note text-text-faint">{say($lang, SOURCE[offer.source])}{found}</span>
    </div>
  {/if}
  <code class="block min-w-0 break-all rounded-control bg-page px-snug py-tight font-mono text-note text-text">
    {offer.launch_preview}
  </code>
  <dl class="flex flex-wrap gap-x-wide gap-y-tight text-note">
    {#if !named}
      <div class="flex gap-tight">
        <dt class="text-text-faint">{say($lang, "acp_source")}</dt>
        <dd class="text-text-quiet">{say($lang, SOURCE[offer.source])}</dd>
      </div>
    {/if}
    <div class="flex gap-tight">
      <dt class="text-text-faint">{say($lang, "acp_version")}</dt>
      <dd class="text-text-quiet">{pinned === undefined ? version : `${version} (${say($lang, pinned)})`}</dd>
    </div>
    <div class="flex gap-tight">
      <dt class="text-text-faint">{say($lang, "acp_licence")}</dt>
      <dd class="text-text-quiet">{offer.licence ?? say($lang, "acp_licence_unstated")}</dd>
    </div>
    <div class="flex gap-tight">
      <dt class="text-text-faint">{say($lang, "acp_env")}</dt>
      <dd class="font-mono text-text-quiet">
        {offer.env_names.length === 0 ? say($lang, "acp_env_none") : offer.env_names.join(" ")}
      </dd>
    </div>
    <div class="flex gap-tight">
      <dt class="text-text-faint">{say($lang, "acp_login")}</dt>
      <dd class="text-text-quiet">{say($lang, loginKey(offer.login))}</dd>
    </div>
  </dl>
  <p class="text-note text-text-quiet">{say($lang, "acp_consent")}</p>
  <div class="flex flex-wrap gap-snug pt-tight">
    {#if seats}
      <Button label={say($lang, "acp_add_and_use")} tone="primary" loading={adding} onPress={() => {
          onAdd(offer, true);
        }} />
    {/if}
    <Button
      label={say($lang, adding ? "acp_adding" : "acp_add_only")}
      tone={seats ? "quiet" : "primary"}
      loading={adding}
      onPress={() => {
        onAdd(offer, false);
      }}
    />
  </div>
</div>
