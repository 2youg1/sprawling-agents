<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The states of the Mayor room's box that only show once a person
  // acts: each of the four pills with its menu open, the coin key's
  // three faces and the microphone's states beside the box, the lines
  // a drop leaves under the box for files it did not keep, and the card
  // a refused dispatch leaves where the reply would have been.
  //
  // Each menu opens upward from the foot of its case, as it does over
  // the box at the foot of the window, so a case reserves the height
  // of the longest menu above its trigger (`pt-menu` below); a menu
  // drawn outside its case is what `xtask render` refuses.
  import type { AxError } from "../../wire";
  import { Address } from "../../wire";

  const HERE = Address.make("hall/mayor");

  const NO_MODEL: AxError = {
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    action: "choose the main model",
    code: "E_MODEL_UNCHOSEN",
    nearby: [],
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "attach a provider on the settings page and pick a model for this tag",
    retry: "no",
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    subject: "no model is chosen for this tag",
  };
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { workspacePill } from "../talk/composer";
  import PillView from "../talk/pill.svelte";
  import Failed from "../talk/failed.svelte";
  import Coin from "../talk/coin.svelte";
  import DropRefused from "../talk/drop_refused.svelte";
  import type { Refused } from "../talk/dropping";
  import { lookOf as recordLook } from "../talk/record";
  import Record from "../talk/record.look.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();

  const ignore = (): void => undefined;
  const workspace = $derived(workspacePill($lang, { rooms: ["hall/mayor", "atlas/api", "atlas/web"], here: HERE }, ignore));
  const NAMES = ["workspace"] as const;
  const FACES = ["send", "stop", "idle"] as const;
  // The microphone is drawn from its look: its seat records through the
  // browser, which a fixture has no microphone for.
  const HEARD = [
    { taking: false, hearing: false, refused: false },
    { taking: true, hearing: false, refused: false },
    { taking: false, hearing: true, refused: false },
    { taking: false, hearing: false, refused: true },
  ] as const;
  // One reason the city gave, and one drop whose answer never came back.
  const NOT_KEPT: readonly Refused[] = [
    // wording-ok: fixture states the city's own refusal, English as the city writes it
    { kind: "refused", name: "report.pdf", said: "the file is larger than the city keeps" },
    { kind: "refused", name: "notes.md", said: "" },
  ];
</script>

{#each NAMES as name (name)}
  <Case label={`composer pill · the ${name} menu open`}>
    <div class="flex min-h-[36rem] items-end">
      <PillView spec={workspace} starts="open" />
    </div>
  </Case>
{/each}

<Case label="coin key · send, stop and the faint face that says why">
  <div class="flex items-center gap-base">
    {#each FACES as face (face)}
      <Coin {face} sending="dispatch" onStop={ignore} />
    {/each}
  </div>
</Case>

<Case label="microphone · ready, recording, hearing, refused">
  <div class="flex flex-wrap items-center gap-base">
    {#each HEARD as heard, index (index)}
      <Record {...recordLook(heard, $lang, ignore)} />
    {/each}
  </div>
</Case>

<Case label="composer · files a drop did not keep">
  <DropRefused refused={NOT_KEPT} />
</Case>

<Case label="conversation · a dispatch refused for want of a model">
  <Failed what={say($lang, "talk_failed_model")} error={NO_MODEL} onRetry={ignore} />
</Case>
