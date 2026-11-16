<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How one line under RefRain's head is drawn (client/Spec.lean §7N): why
  // the text is read only, why a save's outcome is not known yet, what
  // the city refused, that a draft is not in the preview. Each is a full
  // width line in the note size with a hairline under it, so a stack of
  // them reads as part of the head rather than as the document's words.
  import type { LineLook } from "./line";

  const { tone, children }: LineLook = $props();
</script>

<div class="line" data-tone={tone}>{@render children()}</div>

<style>
  .line {
    padding: var(--spacing-tight) var(--spacing-wide);
    border-bottom: 1px solid var(--color-edge);
    font-size: var(--text-note);
    color: var(--color-text-faint);
  }
  .line[data-tone="alert"] {
    color: var(--color-alert);
  }
</style>
