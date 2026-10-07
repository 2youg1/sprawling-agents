<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the page banner (`./banner`): it decides the strip's
  // role from its weight and draws whatever `./banner.look.svelte` is.
  import { bannerOf } from "./banner";
  import type { BannerProps } from "./banner";
  import Look from "./banner.look.svelte";

  const props: BannerProps = $props();
</script>

<Look {...bannerOf(props)} />
