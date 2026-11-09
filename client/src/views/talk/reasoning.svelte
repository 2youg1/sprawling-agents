<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat for a round's reasoning (client/Spec.lean D95): a growing
  // string is painted at most once per animation frame, so a fast model on
  // a 60 Hz or 360 Hz display follows the display refresh rather than the
  // token rate. Nothing else here depends on frames, so the value
  // `./reasoning.ts` builds stays a plain function of the painted text.
  import { ui } from "../../ui";
  import { lookOf } from "./reasoning";
  import Look from "./reasoning.look.svelte";

  interface Props {
    readonly text: string;
    readonly open: boolean;
  }

  const { text, open }: Props = $props();
  const { lang } = ui();

  // `null` until the first frame commits, so the look reads `text`
  // directly on the opening paint without freezing a `$state(text)`.
  let shown = $state<string | null>(null);
  $effect(() => {
    const next = text;
    const id = requestAnimationFrame(() => {
      shown = next;
    });
    return () => {
      cancelAnimationFrame(id);
    };
  });

  const look = $derived(lookOf($lang, shown ?? text, open));
</script>

<Look {...look} />
