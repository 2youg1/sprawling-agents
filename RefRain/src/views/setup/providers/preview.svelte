<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Draft } from "./draft";

  export interface PreviewsProps {
    readonly draft: Draft;
    readonly reference: string;
    readonly model: string;
  }
</script>

<script lang="ts">
  import { ui } from "../../../ui";
  import { readingsOf } from "./preview";
  import Look from "./preview.look.svelte";

  const { draft, reference, model }: PreviewsProps = $props();
  const { lang } = ui();
  const readings = $derived(readingsOf(draft, reference, model, $lang));
</script>

<Look {readings} />
