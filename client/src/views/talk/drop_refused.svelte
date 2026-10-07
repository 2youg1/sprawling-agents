<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the files a drop on the box did not take: it reads the
  // language and hands `./drop_refused.ts`'s value to whatever
  // `./drop_refused.look.svelte` is.
  import { ui } from "../../ui";
  import type { Refused } from "./dropping";
  import { lookOf } from "./drop_refused";
  import Look from "./drop_refused.look.svelte";

  const { refused }: { readonly refused: readonly Refused[] } = $props();
  const { lang } = ui();
</script>

<Look {...lookOf(refused, $lang)} />
