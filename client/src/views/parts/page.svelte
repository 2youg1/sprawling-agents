<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the page frame (`./page`): it decides which heading
  // takes focus and draws whatever `./page.look.svelte` is.
  import { pageOf } from "./page";
  import type { PageProps } from "./page";
  import Look from "./page.look.svelte";

  const props: PageProps = $props();
</script>

<Look {...pageOf(props)} />
