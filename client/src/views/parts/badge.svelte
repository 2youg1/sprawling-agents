<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the badge: it reads which of the three forms it was
  // given (`./badge`) and draws whatever `./badge.look.svelte` is.
  import { badgeOf } from "./badge";
  import type { AnyBadgeProps } from "./badge";
  import Look from "./badge.look.svelte";

  const props: AnyBadgeProps = $props();

  const look = $derived(badgeOf(props));
</script>

{#if look !== undefined}
  <Look {...look} />
{/if}
