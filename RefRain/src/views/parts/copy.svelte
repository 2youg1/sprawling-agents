<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { onDestroy } from "svelte";

  import { ui } from "../../ui";
  import { lookOf, presser, REST, textOf } from "./copy";
  import type { Copied, CopyProps } from "./copy";
  import Look from "./copy.look.svelte";

  const props: CopyProps = $props();
  const { lang } = ui();

  let copied: Copied = $state(REST);
  
  
  let waiting: ReturnType<typeof setTimeout> | undefined = undefined;
  onDestroy(() => {
    clearTimeout(waiting);
  });

  const press = presser({
    board: () => navigator.clipboard,
    later: (run, ms) => {
      const held = setTimeout(run, ms);
      waiting = held;
      return () => {
        clearTimeout(held);
      };
    },
    show: (now) => {
      copied = now;
    },
  });

  const look = $derived(
    lookOf(props, copied, $lang, () => {
      press(textOf(props.text));
    }),
  );
</script>

<Look {...look} />
