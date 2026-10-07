<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the one icon-only key: callers import this file, and it
  // draws whatever `./icon_button.look.svelte` is with the value
  // `./icon_button.ts` builds from the props.
  import { lookOf, type IconButtonProps } from "./icon_button";
  import Look from "./icon_button.look.svelte";

  const props: IconButtonProps = $props();
  const look = $derived(lookOf(props));
</script>

<Look {...look} />
