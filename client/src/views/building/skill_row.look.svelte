<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<!-- One skill, in two lines: what the skill is (its name and the summary
its SKILL.md opens with), then where it lives - the shelf, the file on
it, and the two qualifiers - so a person who finds a name twice can see
which copy runs read. A row that opens takes the row's wash under the
pointer; a disabled one stays as it is, because nothing would answer the
press.

The two trailing qualifiers shrink; they do not hold their width
against the row. The name and the shelf keep their width because they
are what identifies the row; whether it is admitted and who pinned it
are qualifiers, and a qualifier that has to be cut short is still
read. -->
<script lang="ts">
  import type { SkillRowLook } from "./skill_row";

  const look: SkillRowLook = $props();
</script>

<button
  {...look.wire}
  class="flex w-full min-w-0 flex-col gap-tight rounded-control px-snug py-snug text-left text-note enabled:hover:wash disabled:cursor-default"
>
  <span class="flex w-full min-w-0 items-baseline gap-base">
    <span class="shrink-0 font-mono text-text-quiet">{look.name}</span>
    <span class="min-w-0 flex-1 truncate text-text-faint">{look.summary}</span>
  </span>
  <span class="flex w-full min-w-0 items-baseline gap-base">
    <span class="shrink-0 text-text-faint">{look.shelf}</span>
    <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{look.at}</span>
    <span class="min-w-0 truncate text-text-faint">{look.admitted}</span>
    <span class="min-w-0 truncate text-text-faint">{look.used}</span>
  </span>
</button>
