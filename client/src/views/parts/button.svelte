<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the one button in the client: callers import this file,
  // and it draws whatever `./button.look.svelte` is with the value
  // `./button.ts` builds from the props. What a press does, and when it
  // may not land, is decided there and nowhere here.
  import { lookOf, type ButtonProps } from "./button";
  import Look from "./button.look.svelte";

  const props: ButtonProps = $props();
  const look = $derived(lookOf(props));
</script>

<Look {...look} />
