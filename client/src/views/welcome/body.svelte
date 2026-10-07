<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What one step of the guide opens onto: the door the city already has
  // for that job, mounted here rather than copied, so the guide and the
  // settings cannot come to disagree about how a provider is attached or
  // a document saved. Each door asks its own questions only while its
  // step is open.

  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import type { GuideStep } from "../../wire";
  import Machine from "../machine.svelte";
  import GovernedSection from "../setup/governed.svelte";
  import ModelChoice from "../setup/models.svelte";
  import SkillsSection from "../setup/skills.svelte";
  import ProviderDoor from "../shared/provider.svelte";
  import { mcpDoorOf } from "./door";
  import Door from "./door.look.svelte";

  interface Props {
    readonly step: GuideStep;
  }

  const { step }: Props = $props();
  const u = ui();
  const { lang } = u;

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const answer = $derived(readAnswer($endpoints, (held) => ("endpoints" in held ? held.endpoints : undefined)));
</script>

{#if step === "provider"}
  <div class="flex min-w-0 max-w-talk flex-col gap-wide">
    <ProviderDoor />
    {#if answer.kind === "held" && answer.value.endpoints.length > 0}
      <ModelChoice answer={answer.value} tags={["main"]} />
    {/if}
  </div>
{:else if step === "dependencies"}
  <Machine />
{:else if step === "texts"}
  <GovernedSection />
{:else if step === "skills"}
  <SkillsSection />
{:else if step === "mcp"}
  <Door {...mcpDoorOf($lang)} />
{/if}
