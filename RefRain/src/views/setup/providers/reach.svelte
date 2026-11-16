<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import { refusalOf, stoppedAt } from "../../../core/probed";
  import type { Probed } from "../../../core/probed";
  import type { Key } from "../../../core/lang";
  import type { AxCode, AxError } from "../../../wire";

  export interface ReachProps {
    readonly probed: Probed | null;
    readonly error: AxError | null;
    readonly host: string;
  }

  const NEXT_STEP: readonly (readonly [AxCode, Key])[] = [
    ["E_PROVIDER", "setup_next_provider"],
    ["E_TIMEOUT", "setup_next_timeout"],
    ["E_CREDENTIAL_MISSING", "setup_next_credential"],
    ["E_CONFIG_INVALID", "setup_next_config"],
    ["E_INVALID_ARGS", "setup_next_config"],
    ["E_ENDPOINT_DIALECT_UNSUPPORTED", "setup_next_dialect"],
    ["E_WIRE_MISMATCH", "setup_next_dialect"],
  ];

  function nextStep(code: AxCode): Key {
    return NEXT_STEP.find(([known]) => known === code)?.[1] ?? "setup_next_other";
  }
</script>

<script lang="ts">
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import Tip from "../../parts/tip.svelte";

  const { probed, error, host }: ReachProps = $props();
  const { lang } = ui();

  const reach = $derived(probed === null ? null : probed.reach);

  const stages = $derived(
    reach === null
      ? []
      : [
          ["setup_reach_dns", reach.named] as const,
          ["setup_reach_tcp", reach.connected] as const,
          ["setup_reach_http", reach.answered] as const,
          ["setup_reach_proxy", reach.through] as const,
        ],
  );
</script>

{#if error !== null}
  <div role="alert" class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note">
    {#if refusalOf(error.code) === "reach"}
      <p class="font-label text-alert">
        {fill(say($lang, "setup_probe_failed"), { host })}
      </p>
    {:else}
      <p class="font-label text-alert">{say($lang, "setup_form_refused")}</p>

      <p class="mt-tight font-mono wrap-anywhere text-text">{error.subject}</p>
    {/if}
    <p class="mt-tight text-text-quiet">{say($lang, nextStep(error.code))}</p>
    <div class="mt-snug flex items-center gap-snug text-text-faint">
      <span class="font-mono">{error.code}</span>
      <Button
        label={say($lang, "setup_copy_details")}
        tone="quiet"
        onPress={() => {
          void navigator.clipboard.writeText(
            `${error.code}\n${error.action}\n${error.subject}\n${error.recovery}`,
          );
        }}
      />
    </div>
  </div>
{/if}

{#if probed !== null && reach !== null}
  {@const found = reach}
  {@const failed = probed.failure}
  <div
    role="status"
    class={[
      "rounded-card border bg-chrome px-base py-snug text-note",
      failed === null ? "border-edge-panel" : "border-alert/50",
    ]}
  >
    <p class={["font-label", failed === null ? "text-text" : "text-alert"]}>
      {fill(say($lang, "setup_probe_read"), { host: found.host })}
    </p>
    <p class="mt-tight text-text-quiet">{say($lang, stoppedAt(found, failed))}</p>
    <dl class="mt-snug grid grid-cols-2 gap-x-base gap-y-tight text-text-faint">
      {#each stages as [word, stage] (word)}
        <dt>{say($lang, word)}</dt>

        <dd class="min-w-0 font-mono">
          <Tip text={stage.detail ?? stage.state}>
            {#snippet children(hint: string)}
              <!-- svelte-ignore a11y_no_noninteractive_tabindex (a reading is not a control, but only a focusable node can summon its hint from the keyboard, which is what WCAG 1.4.13 owes this stage) -->
              <span class="wrap-anywhere" tabindex="0" aria-describedby={hint}>
                {stage.figure === null ? stage.state : `${stage.state} ${String(stage.figure)}`}
              </span>
            {/snippet}
          </Tip>
        </dd>
      {/each}
    </dl>
    <div class="mt-snug flex items-center gap-snug text-text-faint">
      <span>{fill(say($lang, "setup_reach_elapsed"), { ms: String(found.elapsedMs) })}</span>
      {#if failed !== null}
        <span class="font-mono">{failed.code}</span>
        <Button
          label={say($lang, "setup_copy_details")}
          tone="quiet"
          onPress={() => {
            void navigator.clipboard.writeText(`${failed.code} ${failed.subject}`);
          }}
        />
      {/if}
    </div>
  </div>
{/if}
