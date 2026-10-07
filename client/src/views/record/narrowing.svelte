<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of one narrowing choice: it lends the decision
  // (`./narrowing`) the person's language and draws whatever
  // `./narrowing.look.svelte` is.

  import { ui } from "../../ui";
  import { lookOf } from "./narrowing";
  import type { NarrowingProps } from "./narrowing";
  import Look from "./narrowing.look.svelte";

  const props: NarrowingProps = $props();
  const lang = ui().lang;
  const look = $derived(lookOf(props, $lang));
</script>

<Look {...look} />
