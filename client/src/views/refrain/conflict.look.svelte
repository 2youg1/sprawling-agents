<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the conflict bar is drawn (client/Spec.lean §7N): one sentence and
  // its three actions on one line, carrying the `asks` mark of everything
  // that waits on a person (docs/frontend-method.md §7C). Where it stands
  // above the editor is the document's to say, so the bar writes no
  // margin of its own.
  import type { ConflictLook } from "./conflict";

  const look: ConflictLook = $props();
</script>

<div {...look.wire} class="bar asks">
  <p class="min-w-0 flex-1">{look.text}</p>
  {@render look.actions()}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-snug);
    font-size: var(--text-note);
    color: var(--color-text);
  }
</style>
