<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Where a dispatch landed, seen from the room it was sent from, in the
  // docked composer it sits over. A run that started in this room draws
  // no line, because the thread above already holds it; a run that
  // started in a room of its own leaves one line naming that room as a
  // link (client D6). The room name is Chinese on purpose: the
  // link has to carry a name the address bar percent-encodes.
  import type { Landing } from "../../core/landing";
  import { say } from "../../core/lang";
  import { Address, RunId } from "../../wire";
  import { ui } from "../../ui";
  import Composer from "../talk/composer.svelte";
  import Landed from "../talk/landed.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();

  const LANDINGS: readonly (readonly [string, Landing])[] = [
    ["dispatch · the run started in the room it was sent from", { kind: "here" }],
    [
      "dispatch · the run started in a room of its own",
      {
        kind: "elsewhere",
        run: RunId.make("00000000-0000-4000-8000-00000000f011"),
        addr: Address.make("shop/给-price-加一个测试"),
      },
    ],
  ];
</script>

{#each LANDINGS as [label, landing] (label)}
  <Case {label}>
    <div class="px-pane pb-pane">
      <Landed {landing} />
      <Composer
        placeholder={say($lang, "talk_placeholder_mayor")}
        sending="dispatch"
        hearing
        onSend={() => false}
        onStop={() => false}
      />
    </div>
  </Case>
{/each}
