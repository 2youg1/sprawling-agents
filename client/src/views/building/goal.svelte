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
  import Field from "../parts/field.svelte";

  interface Props {
    readonly address: Address;
  }

  const { address }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const city = u.conn.asking.ask(QUERIES.city);

  let goal = $state("");

  const pursuit = $derived.by(() => {
    const held = $city;
    return held !== undefined && "city" in held
      ? held.city.pursuits.find((line) => line.addr === address)
      : undefined;
  });

  // An empty goal sends nothing; the press moves the caret to the box,
  // so the person sees where the goal is missing. The box is the form's
  // one input, so the form is where the caret is sent from.
  function setGoalNow(form: HTMLFormElement): void {
    const words = goal.trim();
    if (words === "") {
      form.querySelector("input")?.focus();
    } else if (u.send(pursue(address, { set: { goal: words } }))) {
      goal = "";
    }
  }
</script>

<div class="flex min-w-0 items-center gap-base" role="group" aria-label={say($lang, "bld_goal")}>
  <span class="w-figure shrink-0 text-note text-text-faint">{say($lang, "bld_goal")}</span>
  {#if pursuit === undefined}
    <!-- The box is the one every other input is, and its button is
         always there, so a person reads it as a place to type rather
         than as a label. A form, so Enter in the box is the button. -->
    <form
      class="flex min-w-0 flex-1 items-center gap-base"
      onsubmit={(event) => {
        event.preventDefault();
        setGoalNow(event.currentTarget);
      }}
    >
      <div class="min-w-0 flex-1">
        <Field
          label={say($lang, "bld_goal")}
          labelling="hidden"
          placeholder={fill(say($lang, "bld_goal_placeholder"), { addr: address })}
          value={goal}
          onInput={(value) => {
            goal = value;
          }}
        />
      </div>
      <Button label={say($lang, "bld_pursue")} type="submit" tone={goal.trim() === "" ? "secondary" : "primary"} />
    </form>
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
