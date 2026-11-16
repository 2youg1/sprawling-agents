<script lang="ts" generics="T">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

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
