<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // The Rust tools pack: every cargo subcommand this repository calls,
  // drawn as one row with one install control (sprawling-SPEC §8-58).
  //
  // Each member is still its own item on the wire - detected, judged and
  // installed on its own - so the one press hands the page's install walk
  // the pack's missing members in table order, and the walk's steps show
  // each one's progress and, when it fails, the log it wrote. A member
  // this city cannot install here (kani has no Windows build) says so
  // under its name rather than joining the walk.

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { DoctorItem, DoctorNewest } from "../../wire";
  import { offerOf, spelledOf, stateKey } from "../setup/dependencies";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Versions from "./versions.svelte";

  interface Props {
    readonly members: readonly DoctorItem[];
    readonly newest: Readonly<Record<string, DoctorNewest>>;
    readonly walking: boolean;
    readonly onInstall?: ((names: readonly string[]) => void) | undefined;
  }

  const { members, newest, walking, onInstall }: Props = $props();
  const { lang } = ui();

  const pressable = $derived(members.filter((each) => offerOf(each) === "press").map((each) => each.name));
  const wanting = $derived(
    members.some((each) => !("present" in each.state) && each.need === "required"),
  );
  // The pack is complete when every member this city can install here
  // is here; a member only a person can install (kani on Windows) says
  // so on its own line rather than keeping the whole pack "absent".
  const complete = $derived(members.every((each) => "present" in each.state || offerOf(each) === "by_hand"));
</script>

<li class="col-span-full flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
  <div class="flex min-w-0 flex-wrap items-center gap-snug">
    <span class="shrink-0 text-label font-label text-text">{say($lang, "machine_pack_rust_tools")}</span>
    <Badge
      text={say($lang, complete ? "machine_present" : "machine_absent")}
      weight={wanting ? "alert" : "quiet"}
      dot
    />
    <span class="min-w-0 flex-1"></span>
    {#if pressable.length > 0 && onInstall !== undefined}
      <Button
        label={fill(say($lang, "machine_pack_install"), { count: String(pressable.length) })}
        tone="secondary"
        loading={walking}
        onPress={() => {
          onInstall(pressable);
        }}
      />
    {/if}
  </div>
  <p class="text-note text-text-faint">{say($lang, "machine_pack_rust_tools_note")}</p>
  {#if pressable.length > 0}
    <p class="text-note text-text-faint">{say($lang, "machine_pack_install_note")}</p>
  {/if}
  <ul class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-x-wide gap-y-snug">
    {#each members as member (member.name)}
      {@const how = spelledOf(member.install)}
      <li class="flex min-w-0 flex-col gap-tight border-t border-edge pt-snug">
        <div class="flex min-w-0 items-center gap-snug">
          {#if member.homepage === undefined || member.homepage === null}
            <span class="shrink-0 font-mono text-label text-text">{member.name}</span>
          {:else}
            <a
              class="shrink-0 font-mono text-label text-text underline decoration-edge underline-offset-2 hover:decoration-accent"
              href={member.homepage}
              target="_blank"
              rel="noreferrer">{member.name}</a
            >
          {/if}
          <Badge
            text={say($lang, stateKey(member.state))}
            weight={"present" in member.state || member.need === "optional" ? "quiet" : "alert"}
            dot
          />
          {#if member.need === "optional"}
            <span class="shrink-0 text-note text-text-faint">{say($lang, "machine_optional")}</span>
          {/if}
        </div>
        <Versions item={member} newest={newest[member.name]} />
        {#if !("present" in member.state) && how !== null && offerOf(member) !== "press"}
          <p class="min-w-0 break-words text-note text-text-faint">{how}</p>
        {/if}
      </li>
    {/each}
  </ul>
</li>
