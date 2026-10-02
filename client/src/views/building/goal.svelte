<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The building's standing goal, one line under the page header: a box
  // to give it one, or the goal it is pursuing with the verdict of its
  // last step and the two verbs that pause it and take it away. The
  // city's pursuit line is the one authority; this line draws it.
  import { QUERIES } from "../../core/asking";
  import { pursue } from "../../core/commands";
  import { fill, say } from "../../core/lang";
  import { pursuitClause } from "../../core/pursuit";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Button from "../parts/button.svelte";

  interface Props {
    readonly address: Address;
  }

  const { address }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const city = u.conn.asking.ask(QUERIES.city);

  let goal = $state("");
  let field = $state<HTMLInputElement | undefined>(undefined);

  const pursuit = $derived.by(() => {
    const held = $city;
    return held !== undefined && "city" in held
      ? held.city.pursuits.find((line) => line.addr === address)
      : undefined;
  });

  // An empty goal sends nothing; the press moves the caret to the box,
  // so the person sees where the goal is missing.
  function setGoalNow(): void {
    const words = goal.trim();
    if (words === "") {
      field?.focus();
    } else if (u.send(pursue(address, { set: { goal: words } }))) {
      goal = "";
    }
  }
</script>

<div class="flex min-w-0 items-center gap-base" role="group" aria-label={say($lang, "bld_goal")}>
  <span class="w-figure shrink-0 text-note text-text-faint">{say($lang, "bld_goal")}</span>
  {#if pursuit === undefined}
    <!-- The box wears the same line as every other input, and its button
         is always there, so a person reads it as a place to type rather
         than as a label. -->
    <input
      class="h-control min-w-0 flex-1 rounded-control border border-edge-input bg-raised px-base text-note placeholder:text-text-faint"
      placeholder={fill(say($lang, "bld_goal_placeholder"), { addr: address })}
      aria-label={say($lang, "bld_goal")}
      bind:value={goal}
      bind:this={field}
      onkeydown={(event) => {
        if (event.key === "Enter") setGoalNow();
      }}
    />
    <Button
      label={say($lang, "bld_pursue")}
      tone={goal.trim() === "" ? "secondary" : "primary"}
      onPress={setGoalNow}
    />
  {:else}
    <span
      class={["size-dot shrink-0 rounded-pill", pursuit.state === "running" ? "bg-accent" : "bg-mark"]}
      aria-hidden="true"
    ></span>
    <span class="min-w-0 flex-1 truncate text-note text-text">{pursuit.goal}</span>
    <span class="shrink-0 text-note text-text-faint">{pursuitClause($lang, pursuit.verdict)}</span>
    <Button
      label={pursuit.state === "running" ? say($lang, "bld_pause") : say($lang, "bld_resume")}
      tone="quiet"
      onPress={() => {
        u.send(pursue(address, pursuit.state === "running" ? "pause" : "resume"));
      }}
    />
    <Button
      label={say($lang, "bld_clear")}
      tone="quiet"
      onPress={() => {
        u.send(pursue(address, "clear"));
      }}
    />
  {/if}
</div>
