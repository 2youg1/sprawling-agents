<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { PopoverColumn } from "../../parts/popover";
  import Popover from "../../parts/popover.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();

  function forkColumns(): readonly PopoverColumn[] {
    return [
      {
        id: "turns",
        label: "run_turns",
        rows: [
          { id: "turn-3", label: fill(say($lang, "run_turn_n"), { n: "3" }), secondary: "03:35" },
          {
            id: "turn-2",
            label: fill(say($lang, "run_turn_n"), { n: "2" }),
            secondary: "03:12",
            chosen: true,
          },
          { id: "turn-1", label: fill(say($lang, "run_turn_n"), { n: "1" }), secondary: "02:58" },
        ],
      },
      {
        id: "calls",
        label: "talk_calls",
        rows: [
          { id: "call-11", label: "exec: cargo test -p sprawling-kernel", chosen: true },
          { id: "call-10", label: "edit: Sprawling/kernel/src/gate/door.rs" },
          { id: "call-4", label: "search: GateOutcome" },
        ],
      },
    ];
  }
</script>

<Case label="popover · two columns of one choice">
  <div class="pt-palette">
    <div class="pt-output">
      <div class="relative">
        <Popover
          label="fork_pick_title"
          columns={forkColumns()}
          onApply={() => undefined}
          onClose={() => undefined}
        />
      </div>
    </div>
  </div>
</Case>

<Case label="popover · under the top edge of a box that clips it, opening downward">
  <div class="h-palette overflow-hidden">
    <div class="relative">
      <Popover
        label="fork_pick_title"
        columns={forkColumns()}
        onApply={() => undefined}
        onClose={() => undefined}
      />
    </div>
  </div>
</Case>
