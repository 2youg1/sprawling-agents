<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- Where a note sits in the thread's column, the one place its outer
space is written (`Rhythm`, `./note`): a look draws the note and keeps
no margin of its own, so a look put in its place lands on the same
eight-pixel baseline. Any other attribute the seat names, such as the
echo's `data-local-feedback`, is set on this box. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";
  import type { Rhythm } from "./note";

  interface Props extends HTMLAttributes<HTMLDivElement> {
    readonly rhythm: Rhythm;
    readonly children: Snippet;
  }

  const { rhythm, children, ...rest }: Props = $props();

  const SPACE: Readonly<Record<Rhythm, string>> = {
    line: "my-snug",
    shape: "my-pane",
  };
</script>

<div class={SPACE[rhythm]} {...rest}>
  {@render children()}
</div>
