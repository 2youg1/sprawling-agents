<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The states of the Mayor room's box that only show once a person
  // acts: the coin key's three faces and the microphone's states beside
  // the box, the lines a drop leaves under the box for files it did not
  // keep, and the card a refused dispatch leaves where the reply would
  // have been.
  import type { AxError } from "../../wire";


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
  import Failed from "../talk/failed.svelte";
  import Coin from "../talk/coin.svelte";
  import DropRefused from "../talk/drop_refused.svelte";
  import type { Refused } from "../talk/dropping";
  import { lookOf as recordLook } from "../talk/record";
  import Record from "../talk/record.look.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();

  const ignore = (): void => undefined;
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
