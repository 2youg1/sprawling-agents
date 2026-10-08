<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Readable } from "svelte/store";

  import type { Lang } from "../core/lang";
  import type { Credential, Paired, PairWhy } from "../core/local/entering";

  // What a browser with no session sees on this machine's port
  // (client/Spec.lean §4-57b): where the code is, one large box for it,
  // the name this browser will be listed under, and one button. No
  // countdown: the code is good until somebody guesses it wrong.
  export interface PairingProps {
    readonly why: PairWhy;
    readonly lang: Readable<Lang>;
    // The name the box starts with: the browser and its system.
    readonly label: string;
    // Sends one pairing; the caller knocks on the door.
    readonly pair: (code: string, label: string) => Promise<Paired>;
    readonly onPaired: (credential: Credential) => void;
    // `page` is the whole document's one page; `specimen` is the same
    // form in a gallery fold, under the gallery's own heading and landmark.
    readonly seat?: "page" | "specimen";
  }
</script>

<script lang="ts">
  import { say } from "../core/lang";
  import Button from "./parts/button.svelte";
  import Field from "./parts/field.svelte";
  import { codeOf, WHY } from "./pairing";

  const { why, lang, label, pair, onPaired, seat = "page" }: PairingProps = $props();
  const uid = $props.id();
  const sentence = $derived(WHY[why]);

  let code = $state("");
  // svelte-ignore state_referenced_locally
  let named = $state(label);
  let busy = $state(false);
  // The last attempt's failure, already worded.
  let failed = $state<string | null>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const typed = codeOf(code);
    if (typed === "" || busy) return;
    busy = true;
    failed = null;
    const paired = await pair(typed, named.trim() === "" ? label : named.trim());
    busy = false;
    switch (paired.kind) {
      case "paired":
        onPaired(paired.credential);
        return;
      case "refused":
        // One guess per code: the box is emptied for the new one.
        code = "";
        failed = say($lang, "recover_e_pairing_refused");
        return;
      case "unreachable":
        failed = say($lang, "enrol_unreachable");
        return;
    }
  }
</script>

<svelte:element
  this={seat === "page" ? "main" : "div"}
  class={["flex items-center justify-center bg-page px-wide font-sans text-body text-text", seat === "page" ? "min-h-dvh" : "py-wide"]}
>
  <form class="flex w-full max-w-measure flex-col gap-wide" onsubmit={(event) => void submit(event)}>
    <div class="flex flex-col gap-snug">
      <svelte:element this={seat === "page" ? "h1" : "h2"} class="text-title font-title text-text">
        {say($lang, "pair_title")}
      </svelte:element>
      <p class="text-body text-text-quiet">{say($lang, "pair_where")}</p>
      {#if sentence !== undefined}
        <p class="text-body text-text">{say($lang, sentence)}</p>
      {/if}
    </div>
    <div class="flex flex-col gap-snug">
      <label for={`${uid}-code`} class="text-note text-text-quiet">{say($lang, "pair_code")}</label>
      <input
        id={`${uid}-code`}
        bind:value={code}
        autocomplete="one-time-code"
        autocapitalize="none"
        spellcheck="false"
        aria-invalid={failed !== null}
        aria-describedby={failed === null ? undefined : `${uid}-failed`}
        class="h-[4.5rem] w-full rounded-card border border-edge-input bg-raised px-pane text-center font-mono text-figure tracking-[0.2em] text-text"
      />
      {#if failed !== null}
        <p id={`${uid}-failed`} role="alert" class="text-note text-alert">{failed}</p>
      {/if}
    </div>
    <Field
      label={say($lang, "pair_label")}
      help={say($lang, "pair_label_note")}
      value={named}
      onInput={(value: string) => {
        named = value;
      }}
    />
    <div class="flex justify-end">
      <Button
        label={say($lang, busy ? "pair_pairing" : "pair_action")}
        tone="primary"
        type="submit"
        loading={busy}
      />
    </div>
  </form>
</svelte:element>
