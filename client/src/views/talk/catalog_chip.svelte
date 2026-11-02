<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The current building's one catalog count, opening the same unified
  // panel as the building page. No second count is computed here.
  import { readAnswer } from "../../core/answered";
  import { fill, say } from "../../core/lang";
  import { buildingOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, Query } from "../../wire";
  import Button from "../parts/button.svelte";

  interface Props {
    readonly room: Address;
  }

  const { room }: Props = $props();
  const u = ui();
  const { lang } = u;
  const building = $derived(buildingOf(room));
  const question = $derived<Query>({ catalog: { building } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (answer) => ("catalog" in answer ? answer.catalog : undefined)),
  );
  const answer = $derived(read.kind === "held" ? read.value : undefined);
</script>

{#if answer !== undefined}
  <span class="flex min-w-0">
    <Button
      label={fill(say($lang, "catalog_chip"), {
        always: String(answer.always),
        dormant: String(answer.dormant),
        skills: String(answer.skills),
        changed: String(answer.changed),
      })}
      tone="quiet"
      onPress={() => {
        u.go({ kind: "building", address: building, section: "catalog" });
      }}
    />
  </span>
{/if}
