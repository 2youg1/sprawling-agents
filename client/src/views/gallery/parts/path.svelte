<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One printed path in each of its states: text the city can reveal, a
  // path cut by the directory the reader is already in and opening in
  // the page, and a path outside the city whose reveal key says why it
  // cannot be used. The last is in a narrow frame, so the ellipsis shows.
  import Path from "../../parts/path.svelte";
  import Case from "../case.svelte";
</script>

<Case label="path · revealable, and opening in the page">
  <div class="flex flex-col gap-tight">
    <Path path="city/lab/notes.md" />
    <Path
      path="city/lab/west/experiments/report.md"
      base="city/lab"
      onOpen={() => undefined}
    />
  </div>
</Case>

<Case label="path · outside the city, in a narrow column" width={240}>
  <Path path="D:\shared\experiments\2026\07\completions-with-a-long-name.md" />
</Case>
