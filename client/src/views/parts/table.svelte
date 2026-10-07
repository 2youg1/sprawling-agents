<script lang="ts" generics="T">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Many rows of the same shape, chosen among and corrected in place: the
// table a provider's two hundred models are admitted from. A column
// carries its own three abilities - how a cell is drawn, whether the
// column can be ordered by, and whether its value can be corrected in
// place - and the empty state is the caller's snippet, because what to
// do about an empty table is knowledge of the page, not of the table.
//
// The seat: it holds the one piece of state the table owns, which
// column the rows are ordered by, and draws `./table.look.svelte` with
// what `./table` builds from it.

import Look from "./table.look.svelte";
import { UNSORTED, lookOf, turned } from "./table";
import type { Sorted, TableProps } from "./table";

const props: TableProps<T> = $props();

let sorted = $state<Sorted>(UNSORTED);

const look = $derived(
  lookOf(props, sorted, (key) => {
    sorted = turned(sorted, key);
  }),
);
</script>

<Look {...look} />
