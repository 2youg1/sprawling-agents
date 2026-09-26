<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One refusal a turn came to. A failed model call names its kind, and
  // the way out is said from `lang.json` in the reader's language; the
  // city's own sentence stays folded beneath it for whoever is debugging.
  // Any other refusal has only the city's sentence, shown as it is.
  import { say } from "../../core/lang";
  import { providerClause } from "../../core/provider_failure";
  import { ui } from "../../ui";
  import type { AxError } from "../../wire";

  const { error }: { readonly error: AxError } = $props();
  const { lang } = ui();
</script>

<div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
  <span class="text-alert">{error.code}</span> · {error.action} · {error.subject}
  {#if error.provider !== undefined && error.provider !== null}
    <div class="mt-tight text-text-faint">{providerClause($lang, error.provider)}</div>
    {#if error.recovery !== ""}
      <details class="mt-tight text-text-faint">
        <summary class="cursor-pointer">{say($lang, "notices_detail")}</summary>
        <div class="font-mono break-words">{error.recovery}</div>
      </details>
    {/if}
  {:else if error.recovery !== ""}
    <div class="mt-tight text-text-faint">{error.recovery}</div>
  {/if}
</div>
