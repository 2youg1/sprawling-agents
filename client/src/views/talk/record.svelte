<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // The microphone beside the composer. One press records and a second
  // stops; what the city heard comes back as words for the box, and a
  // city that refused to transcribe says so beside the button.
  import { say } from "../../core/lang";
  import { dictation } from "../../core/speaking";
  import { ui } from "../../ui";

  const { onWords }: { readonly onWords: (words: string) => void } = $props();

  const u = ui();
  const { lang } = u;
  const heard = dictation(u.origin, u.pairing, (words) => {
    onWords(words);
  });
  const taking = heard.taking;
  const transcribing = heard.hearing;
  const refused = heard.refused;
</script>

<button
  type="button"
  class={[
    "relative flex h-control-sm shrink-0 items-center gap-tight rounded-pill px-base text-note before:absolute before:-inset-snug before:content-['']",
    $taking ? "bg-alert text-on-accent" : $transcribing ? "bg-raised aria-disabled:text-text-disabled" : "bg-raised text-text-quiet hover:bg-raised-hover",
  ]}
  aria-disabled={$transcribing}
  onclick={() => {
    if ($transcribing) return;
    heard.speak();
  }}
>
  {$transcribing ? say($lang, "talk_hearing") : $taking ? say($lang, "talk_recording") : say($lang, "talk_record")}
</button>
{#if $refused}
  <span class="text-alert">{say($lang, "link_refused")}</span>
{/if}
