<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What came back from the far side, drawn beside the fields it is
  // about. Two panels, because a probe and an attachment fail
  // differently: an attachment that was turned away carries the city's
  // own refusal, and a probe carries a staged reading of how far the
  // call got. The two slots are filled independently, because both can
  // be true of one attempt and dropping one would be this file
  // deciding which failure a person needed to see.
  //
  // The words are the caller's language, read here through `say`,
  // because this panel owns which sentence answers which code and the
  // caller owns when the language changes.

  import { stoppedAt } from "../../../core/probed";
  import type { Probed } from "../../../core/probed";
  import type { Key } from "../../../core/lang";
  import type { AxCode, AxError } from "../../../wire";

  export interface ReachProps {
    readonly probed: Probed | null;
    readonly error: AxError | null;
    // What the fields are about, as the person reads it.
    readonly host: string;
  }

  // What a person can do about a refusal, by the code it carries. The
  // city's own `recovery` speaks to the runtime that must retry or fail
  // over, so it is copied rather than shown; this table is the half
  // written for the person filling in the form. `E_MODEL_UNCHOSEN` has
  // no row: the router's model choice refuses with it, and a reach
  // probe talks to the endpoint without asking the router.
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

  // The call's destination reading, kept once so the stages below and
  // the panel's own gate read the same value.
  const reach = $derived(probed === null ? null : probed.reach);

  // The four stages as one list, so the table below draws from the
  // same order the reading is taken in.
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
  <!-- A refusal the city sent back, which is what an attachment that
  failed produces. A probe answers with a reading instead, and that is
  the other panel below. -->
  <div role="alert" class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note">
    <p class="font-label text-alert">
      {fill(say($lang, "setup_probe_failed"), { host })}
    </p>
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

<!-- Where the call stopped, stage by stage, as the city measured it.
Four stages, each with the word the city wrote for it, and above them
the one sentence a person can act on. The stages are shown even when
the call went through: a host that resolved and answered in 90 ms is
not a proxy problem, and somebody sent to look at one has been sent the
wrong way. The stages ask without the key, so their 401 is read beside
the model list, which was asked with it (`core/probed.ts`). -->
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
        <!-- wording-ok: the word the city itself wrote for this stage -->
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
