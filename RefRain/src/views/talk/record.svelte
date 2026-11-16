<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";

  import { dictation } from "../../core/speaking";
  import { ui } from "../../ui";
  import { lookOf } from "./record";
  import Look from "./record.look.svelte";
  import { speakAsked } from "./speak_asked";

  const { onWords }: { readonly onWords: (words: string) => void } = $props();

  const u = ui();
  const { lang } = u;
  const heard = dictation(u.origin, u.credential, (words) => {
    onWords(words);
  });
  const taking = heard.taking;
  const transcribing = heard.hearing;
  const refused = heard.refused;

  function press(): void {
    if (get(transcribing)) return;
    heard.speak();
  }

  onMount(() => {
    let seen = get(speakAsked);
    return speakAsked.subscribe((count) => {
      if (count === seen) return;
      seen = count;
      press();
    });
  });
</script>

<Look {...lookOf({ taking: $taking, hearing: $transcribing, refused: $refused }, $lang, press)} />
