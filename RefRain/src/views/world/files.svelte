<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { buildingOf } from "../../core/route";
  import type { Address } from "../../wire";
  import Tree from "../building/tree.svelte";
  import { openDocument } from "../inspect/open.svelte";

  interface Props {
    readonly here: Address;
  }

  const { here }: Props = $props();

  const building = $derived(buildingOf(here));
  interface Picked {
    readonly at: Address;
    readonly kind: "file" | "directory";
  }
  let picked = $state<Picked | null>(null);
</script>

<div class="min-h-0 flex-1 overflow-y-auto">
  <Tree
    root={building}
    {picked}
    onPick={(chosen: Picked) => {
      picked = chosen;
      const prefix = `${building}/`;
      if (chosen.kind === "file" && chosen.at.startsWith(prefix)) {
        openDocument({ building, path: chosen.at.slice(prefix.length), version: null });
      }
    }}
  />
</div>
