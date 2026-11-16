<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Ink, InkedLook } from "./code";

  const PAINT: Record<Ink, string> = {
    plain: "",
    comment: "text-text-faint",
    string: "text-alert",
    number: "text-accent",
    word: "text-text",
  };
</script>

<script lang="ts">
  const look: InkedLook = $props();
</script>

{#each look.pieces as piece (piece)}<span class={PAINT[piece.ink]}>{piece.text}</span>{/each}
