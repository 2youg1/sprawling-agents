<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  
  
  import type { Key } from "../core/lang";
  import type { Picked } from "./building/tree.svelte";

  
  const SECTIONS = ["plan", "commits", "changes", "prompts", "catalog", "sandbox"] as const;
  type Section = (typeof SECTIONS)[number];

  const SECTION_WORD: Record<Section, Key> = {
    plan: "bld_plan",
    commits: "bld_commits",
    changes: "bld_changes",
    prompts: "bld_prompts",
    catalog: "bld_catalog",
    sandbox: "bld_sandbox",
  };
</script>

<script lang="ts">
  import { readAnswer } from "../core/answered";
  import { halt, release, removeBuilding } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { removalOf } from "../core/removal";
  import { within } from "../core/belief/live";
  import { buildingIsShut } from "../core/scope";
  import { ui } from "../ui";
  import type { Address, Query } from "../wire";
  import Button from "./parts/button.svelte";
  import Dialog from "./parts/dialog.svelte";
  import Page from "./parts/page.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Commits from "./building/commits.svelte";
  import Directory from "./building/directory.svelte";
  import FileView from "./building/file.svelte";
  import Goal from "./building/goal.svelte";
  import Plan from "./building/plan.svelte";
  import Prompts from "./building/prompts.svelte";
  import Sandbox from "./building/sandbox.svelte";
  import Catalog from "./building/catalog.svelte";
  import Status from "./building/status.svelte";
  import Tree from "./building/tree.svelte";
  import Rooms from "./building/rooms.svelte";
  import { sectionsLookOf } from "./building/sections";
  import Sections from "./building/sections.look.svelte";
  import { openItems, type RightItem } from "./inspect/open.svelte";
  import Right from "./right.svelte";
  import WorkspaceNav from "./shared/workspace.svelte";

  interface Props {
    readonly address: Address;
    
    
    readonly rank?: "page" | "section" | undefined;
    
    
    readonly beside?: readonly RightItem[] | undefined;
    
    readonly initialSection?: "catalog" | undefined;
  }

  const { address, rank = "page", beside = [], initialSection }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  
  let section = $state<Section | null>("plan");
  $effect(() => {
    if (initialSection !== undefined) section = initialSection;
  });
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-redundant-type-constituents -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  let picked = $state.raw<Picked | null>(null);

  const question = $derived<Query>({ building_view: { addr: address } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (answer) => ("building" in answer ? answer.building : undefined)),
  );
  const building = $derived(read.kind === "held" ? read.value : undefined);
  const catalogQuestion = $derived<Query>({ catalog: { building: address } });
  const catalogAsked = $derived(u.conn.asking.ask(catalogQuestion));
  const catalogRead = $derived(
    readAnswer($catalogAsked, (answer) => ("catalog" in answer ? answer.catalog : undefined)),
  );
  const changed = $derived(catalogRead.kind === "held" ? catalogRead.value.changed : 0);

  const halted = $derived(buildingIsShut($belief.halted, address));
  const removal = $derived(removalOf(address, livingIn(address)));
  
  
  let removing = $state(false);
  const done = $derived.by(() => {
    const held = building;
    if (held === undefined || !("planned" in held.progress) || held.progress.planned.total === 0) {
      return null;
    }
    return held.progress.planned;
  });

  
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  const shownAddress: string = $derived(picked?.at ?? "");
  const shownLabel: string = $derived(
    section === null ? shownAddress : say($lang, SECTION_WORD[section]),
  );

  function pick(next: Picked): void {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
    picked = next;
    section = null;
  }

  const opened = $derived(openItems().length > 0 || beside.length > 0);

  const index = $derived(
    sectionsLookOf(
      SECTIONS.map((key) => ({
        key,
        label:
          key === "catalog" && changed > 0
            ? fill(say($lang, "catalog_nav_changed"), { n: String(changed) })
            : say($lang, SECTION_WORD[key]),
      })),
      section,
      (next) => {
        section = next;
        picked = null;
      },
    ),
  );

  
  function livingIn(room: Address): number {
    return $belief.live.filter((run) => within(run, room)).length;
  }
</script>

{#snippet aside()}
  {#if done !== null}
    <span class="figure text-note text-text-quiet">
      {fill(say($lang, "bld_progress"), { done: String(done.done), total: String(done.total) })}
    </span>
  {/if}
  
  {#if !halted && livingIn(address) === 0}
    <span class="text-note text-text-faint">{say($lang, "bld_halt_idle")}</span>
  {/if}
  <Button
    label={fill(say($lang, halted ? "bld_release" : "bld_halt"), { addr: address })}
    tone={halted ? "secondary" : "quiet"}
    onPress={() => {
      u.send(halted ? release({ building: address }) : halt({ building: address }));
    }}
  />
  {#if removal !== "hall"}
    <Button
      label={say($lang, "bld_remove")}
      tone="quiet"
      {...removal === "busy" ? { why: say($lang, "bld_remove_busy") } : {}}
      onPress={() => {
        removing = true;
      }}
    />
  {/if}
{/snippet}

<Page title={address} {rank} {aside}>
  <WorkspaceNav current={address} />
  <Goal {address} />
  
  <div
    class="grid grid-cols-11 grid-rows-[auto_1fr] items-start gap-x-gutter gap-y-wide narrow:grid-cols-1 narrow:grid-rows-none"
  >
    <nav
      class="col-[1/4] row-start-1 min-w-0 narrow:col-span-full narrow:row-auto"
      aria-label={say($lang, "bld_index")}
    >
      <Sections {...index} />
    </nav>
    <section
      class={[
        "row-[1/3] flex min-w-0 flex-col narrow:col-span-full narrow:row-auto",
        opened ? "col-[4/8]" : "col-[4/12]",
      ]}
      aria-label={shownLabel}
    >
      {#if section === "plan"}
        {#if read.kind === "held"}
          <Plan answer={read.value} />
        {:else if read.kind === "unavailable"}
          <Unanswered query={read.query} asked={question} />
        {:else}
          <p class="text-text-faint">…</p>
        {/if}
      {:else if section === "commits"}
        <Commits building={address} />
      {:else if section === "changes"}
        <Status building={address} />
      {:else if section === "prompts"}
        <Prompts building={address} rooms={building?.rooms ?? []} />
      {:else if section === "catalog"}
        <Catalog building={address} />
      {:else if section === "sandbox"}
        <Sandbox {address} held={building?.sandbox ?? null} known={building !== undefined} />
      {:else if picked?.kind === "file"}
        <FileView at={picked.at} root={address} />
      {:else if picked?.kind === "directory"}
        <Directory at={picked.at} root={address} onPick={pick} />
      {/if}
    </section>
    {#if opened}
      <div
        class="sticky top-0 col-[8/12] row-[1/3] flex h-[calc(100dvh-var(--spacing-section)*4)] min-w-0 overflow-hidden rounded-panel border border-edge narrow:col-span-full narrow:row-auto"
      >
        <Right following={beside} current={undefined} talk={address} />
      </div>
    {/if}
    <div
      class="col-[1/4] row-start-2 flex min-w-0 flex-col gap-wide narrow:col-span-full narrow:row-auto"
    >
      <Tree root={address} {picked} onPick={pick} />
      {#if building !== undefined}
        <Rooms
          answer={building}
          living={livingIn}
          onPick={(room) => {
            pick({ at: room, kind: "directory" });
          }}
        />
      {/if}
    </div>
  </div>
</Page>

<Dialog
  open={removing}
  title={fill(say($lang, "bld_remove_title"), { addr: address })}
  detail={say($lang, "bld_remove_detail")}
  confirmLabel={say($lang, "bld_remove")}
  cancelLabel={say($lang, "part_cancel")}
  destructive
  onConfirm={() => {
    removing = false;
    if (u.send(removeBuilding(address))) u.go({ kind: "city" });
  }}
  onCancel={() => {
    removing = false;
  }}
/>
