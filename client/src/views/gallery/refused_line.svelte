<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // A line the city refused, back in the box it was sent from. The
  // fixture types the line and presses Enter the way a person does, and
  // its stand answers the dispatch with the refusal a city without a
  // main model gives, so what is drawn is the composer's own path from
  // send to refusal rather than a box handed its words as a prop.
  import { onMount } from "svelte";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { AxError } from "../../wire";
  import Composer from "../talk/composer.svelte";
  import Case from "./case.svelte";
  import Stand from "./stand.svelte";

  // wording-ok: fixture states what a person typed and the wire's own refusal fields, English as the city writes them
  const LINE = "add a test for price";
  const NO_MODEL: AxError = {
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    action: "choose the main model",
    code: "E_CONFIG_INVALID",
    nearby: [],
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "attach a provider on the settings page and pick a model for this tag",
    retriable: false,
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    subject: "no model is chosen for this tag",
  };

  const { lang } = ui();
  let refusing = $state<AxError | null>(null);
  let holder = $state<HTMLDivElement | undefined>(undefined);

  onMount(() => {
    const box = holder?.querySelector("textarea");
    if (box === null || box === undefined) return;
    box.value = LINE;
    box.dispatchEvent(new Event("input", { bubbles: true }));
    box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  });
</script>

<Case label="composer · a refused line back in the box">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]} {refusing}>
    <div class="px-pane pb-pane" bind:this={holder}>
      <Composer
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        onSend={() => {
          refusing = NO_MODEL;
          return true;
        }}
        onStop={() => false}
      />
    </div>
  </Stand>
</Case>
