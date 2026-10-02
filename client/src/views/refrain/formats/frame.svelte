<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A page drawn apart from the client: an `iframe` whose empty
  // `sandbox` runs no script and gives the page an origin of its own, so
  // nothing it holds can read this page, the city's socket or the
  // browser's storage, and whose page is first closed to the outside by
  // `framed.ts` (client-SPEC 4-54, 12-32). The frame scrolls itself, so
  // a page longer than the pane is read inside it.
  import { framed } from "./framed";

  interface Props {
    readonly page: string;
    readonly scheme: "authored" | "built";
    // The frame's accessible name: what is drawn in it.
    readonly title: string;
  }

  const { page, scheme, title }: Props = $props();

  const shown = $derived(framed(page, scheme));
</script>

<iframe class="min-h-0 w-full flex-1 border-0" sandbox="" referrerpolicy="no-referrer" {title} srcdoc={shown}></iframe>
