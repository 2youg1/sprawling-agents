<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import Look from "./skeleton.look.svelte";
  import { lookOf } from "./skeleton";
  import type { SkeletonProps } from "./skeleton";

  const props: SkeletonProps = $props();
  const look = $derived(lookOf(props));
</script>

<Look {...look} />
