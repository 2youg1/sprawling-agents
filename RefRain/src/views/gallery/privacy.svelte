<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import type { PrivacyControl } from "../../wire";
  import Entry from "../setup/privacy/entry.svelte";
  import { sectionsOf, type EntryModel } from "../setup/privacy/page";
  import Privacy from "../setup/privacy/privacy.svelte";
  import type { Held } from "../setup/privacy/state.svelte";
  import Case from "./case.svelte";
  import { ELSEWHERE, HOME, IDEM, REFUSAL, UNRESOLVED } from "./privacy_answer";
  import SwapLook from "./privacy_swap.look.svelte";

  const MODELS = sectionsOf(HOME).flatMap((section) => section.entries);

  function modelOf(control: PrivacyControl): EntryModel | undefined {
    return MODELS.find((each) => each.row.control === control);
  }

  const STAGED: readonly { label: string; control: PrivacyControl; held: Held }[] = [
    {
      label: "privacy entry · refused, this account may not write an HKCU policy key",
      control: "tailored_experiences",
      held: {
        stage: { kind: "refused", idem: IDEM },
        shown: { refused: { code: "access_denied", error: REFUSAL } },
      },
    },
    {
      label: "privacy entry · waiting, Windows may be asking for administrator approval",
      control: "device_census_task",
      held: { stage: { kind: "sending", action: "apply", idem: IDEM }, shown: null },
    },
    {
      label: "privacy entry · applied and read back, data deleted by Windows on apply",
      control: "recall_snapshots",
      held: { stage: { kind: "settled", idem: IDEM }, shown: { applied: { operation: 1 } } },
    },
    {
      label: "privacy entry · not sent, the link is down",
      control: "powershell_telemetry_optout",
      held: { stage: { kind: "unsent", action: "apply" }, shown: null },
    },
  ];

  const swapped = modelOf("diagnostic_data");
</script>

<Case label="privacy · a Home host, most entries not stated" width={1280}>
  <Privacy fixture={HOME} />
</Case>

<Case label="privacy · one change without a conclusion, only its check offered" width={1280}>
  <Privacy fixture={UNRESOLVED} />
</Case>

<Case label="privacy · a city on another system" width={1280}>
  <Privacy fixture={ELSEWHERE} />
</Case>

{#each STAGED as staged (staged.label)}
  {@const model = modelOf(staged.control)}
  {#if model !== undefined}
    <Case label={staged.label} width={520}>
      <Entry {model} held={staged.held} onPress={() => undefined} />
    </Case>
  {/if}
{/each}

{#if swapped !== undefined}
  <Case label="privacy entry · swapped, the same entry through a second look" width={520}>
    <Entry
      model={swapped}
      held={{ stage: { kind: "idle" }, shown: null }}
      onPress={() => undefined}
      look={SwapLook}
    />
  </Case>
{/if}
