<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  export type { FieldProps } from "./field";
</script>

<script lang="ts">
  // The seat of the one labelled box: callers import this file, and it
  // draws whatever `./field.look.svelte` is with the value `./field.ts`
  // builds from the props and this seat's id. Which description is read
  // out and when an edit may land are decided there and nowhere here.
  import { lookOf, type FieldProps } from "./field";
  import Look from "./field.look.svelte";

  const props: FieldProps = $props();
  const uid = $props.id();
  const look = $derived(lookOf(props, uid));
</script>

<Look {...look} />
