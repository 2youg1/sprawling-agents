<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the run page's sheet: it lends the decision (`./head`)
  // the person's language and draws whatever `./head.look.svelte` is.

  import { ui } from "../../ui";
  import { lookOf } from "./head";
  import type { HeadProps } from "./head";
  import Look from "./head.look.svelte";

  const props: HeadProps = $props();
  const lang = ui().lang;
  const look = $derived(lookOf(props, $lang));
</script>

<Look {...look} />
