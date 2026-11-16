<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { framed } from "./framed";

  interface Props {
    readonly page: string;
    readonly scheme: "authored" | "built";

    readonly title: string;
  }

  const { page, scheme, title }: Props = $props();

  const shown = $derived(framed(page, scheme));
</script>

<iframe
  class="min-h-0 w-full flex-1 border-0"
  sandbox=""
  referrerpolicy="no-referrer"
  {title}
  srcdoc={shown}
></iframe>
