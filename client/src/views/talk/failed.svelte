<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A message that got no reply, drawn where the reply would have been:
  // what happened in the reader's words, the city's own sentence under
  // it, and the next step - the model settings when the model or its
  // provider is what stood in the way, and sending the words again.
  // Nothing a person sent may end in silence.
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { AxError } from "../../wire";
  import Button from "../parts/button.svelte";
  import { settledBySettings } from "./refused";
  import { recoveryWords } from "../parts/notice_title";

  interface Props {
    // What happened, already in the reader's language.
    readonly what: string;
    // The city's refusal, when there is one to show.
    readonly error?: AxError | undefined;
    // Whether the way out goes through the model settings.
    readonly settings?: "offered" | "absent";
    readonly onRetry?: (() => void) | undefined;
  }

  const { what, error, settings, onRetry }: Props = $props();
  const { lang } = ui();

  // A refusal about the model or the provider is fixed in the settings;
  // a caller that knows better says so.
  const nextStep = $derived(error === undefined ? "" : recoveryWords($lang, error.code, error.recovery));

  const toSettings = $derived(
    settings === "offered" || (settings === undefined && error !== undefined && settledBySettings(error)),
  );
</script>

<div class="my-base rounded-card border border-alert/40 px-base py-snug text-note" role="alert">
  <p class="text-alert">{what}</p>
  {#if error !== undefined}
    <p class="mt-tight text-text-quiet">{error.code} · {error.subject}</p>
    {#if nextStep !== ""}
      <p class="mt-tight text-text-faint">{nextStep}</p>
    {/if}
    <!-- The city's own sentence names the case in front of it - which
    field moved, which of two ways out - where the page's word speaks
    for the whole code, so it stays one fold away, as in a notice. -->
    {#if error.recovery !== "" && error.recovery !== nextStep}
      <details class="mt-tight min-w-0">
        <summary class="cursor-pointer text-text-faint">{say($lang, "notices_detail")}</summary>
        <p class="mt-tight wrap-anywhere font-mono text-text-quiet">{error.recovery}</p>
      </details>
    {/if}
  {/if}
  {#if toSettings || onRetry !== undefined}
    <div class="mt-snug flex flex-wrap items-center gap-base">
      {#if toSettings}
        <a href={toFragment({ kind: "setup" })} class="text-text-quiet underline hover:text-text">
          {say($lang, "talk_open_settings")}
        </a>
      {/if}
      {#if onRetry !== undefined}
        <Button label={say($lang, "talk_send_again")} onPress={onRetry} />
      {/if}
    </div>
  {/if}
</div>
