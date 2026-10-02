<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One building, on the shell's column lines (client-SPEC 4-50): the
  // page frame's header names it and carries the verbs that act on the
  // whole building; under it the standing goal; then three columns of
  // the shell's grid - the index and the directory tree on the left,
  // the chosen section or file in the middle, and, when a file of this
  // building is open on the right side, that file on the right.
  //
  // What a person can ask of this building - stop it, set it a standing
  // goal, take it away - is spelled as commands (client-SPEC 4-10): the
  // verbs a person reads are the commands they would type, so the
  // screen teaches the command line rather than a private vocabulary.
  //
  // What the tree hands back crosses a component file boundary as a
  // type. typescript-eslint resolves no named export of a `.svelte`
  // module in its type program, so the type-aware rules below see
  // `Picked` as an error type; `svelte-check` is the type gate and
  // resolves it (client-SPEC 4-1). Each suppression names that gap and
  // nothing else.
  import type { Key } from "../core/lang";
  import type { Picked } from "./building/tree.svelte";

  // The sections of the index, in the order a person reads a building
  // by: what it set out to do, what it committed, what it has changed
  // since, what it knows how to do, and what it is allowed to touch.
  const SECTIONS = ["plan", "commits", "changes", "skills", "sandbox"] as const;
  type Section = (typeof SECTIONS)[number];

  const SECTION_WORD: Record<Section, Key> = {
    plan: "bld_plan",
    commits: "bld_commits",
    changes: "bld_changes",
    skills: "bld_skills",
    sandbox: "bld_sandbox",
  };

</script>

<script lang="ts">
  import { readAnswer } from "../core/answered";
  import { halt, release, removeBuilding } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { removalOf } from "../core/removal";
  import { toFragment } from "../core/route";
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
  import Opened from "./building/opened.svelte";
  import Plan from "./building/plan.svelte";
  import Sandbox from "./building/sandbox.svelte";
  import Skills from "./building/skills.svelte";
  import Status from "./building/status.svelte";
  import Tree from "./building/tree.svelte";
  import Rooms from "./building/rooms.svelte";
  import { rightItem } from "./inspect/open.svelte";

  interface Props {
    readonly address: Address;
    // Whether this is the page or a fixture inside one: a document may
    // have exactly one heading of the page's own rank.
    readonly rank?: "page" | "section" | undefined;
  }

  const { address, rank = "page" }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  // What the middle column shows: a section of the index, or - when
  // `section` is `null` - what the tree picked.
  let section = $state<Section | null>("plan");
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-redundant-type-constituents -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  let picked = $state.raw<Picked | null>(null);

  const question = $derived<Query>({ building_view: { addr: address } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (answer) => ("building" in answer ? answer.building : undefined)));
  const building = $derived(read.kind === "held" ? read.value : undefined);

  const halted = $derived(buildingIsShut($belief.halted, address));
  const removal = $derived(removalOf(address, livingIn(address)));
  // Removing moves the building's files out of the city, so it is asked
  // through `parts/dialog` before the command leaves.
  let removing = $state(false);
  const done = $derived.by(() => {
    const held = building;
    if (held === undefined || !("planned" in held.progress) || held.progress.planned.total === 0) {
      return null;
    }
    return held.progress.planned;
  });


  // What the middle column is called: the address it shows, or the
  // section's word.
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  const shownLabel: string = $derived(section === null ? (picked?.at ?? "") : say($lang, SECTION_WORD[section]));

  function pick(next: Picked): void {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
    picked = next;
    section = null;
  }

  // The right side shows a file of this building, or nothing here: a
  // call, or another building's file, belongs to the conversation's
  // working surface (client-SPEC 12-27).
  const opened = $derived.by(() => {
    const item = rightItem();
    return item !== null && item.kind === "document" && item.building === address ? item : null;
  });

  // How many runs are working at or below a room, which is what the
  // rooms list lights its dots for.
  function livingIn(room: Address): number {
    return $belief.live.filter((run) => within(run, room)).length;
  }
</script>

{#snippet above()}
  <a href={toFragment({ kind: "city" })} class="hover:text-text-quiet">{say($lang, "nav_city")}</a>
  <!-- wording-ok: the path separator between two addresses, hidden from readers -->
  <span aria-hidden="true">/</span>
{/snippet}

{#snippet aside()}
  {#if done !== null}
    <span class="figure text-note text-text-quiet">
      {fill(say($lang, "bld_progress"), { done: String(done.done), total: String(done.total) })}
    </span>
  {/if}
  <!-- Halt still shuts an idle building to new work and ends its backlog
       (glossary: Halt), so the control stays live and only says that
       nothing runs here now. -->
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
      label={fill(say($lang, "bld_remove"), { addr: address })}
      tone="quiet"
      {...removal === "busy" ? { why: say($lang, "bld_remove_busy") } : {}}
      onPress={() => {
        removing = true;
      }}
    />
  {/if}
{/snippet}

<Page title={address} {rank} {above} {aside}>
  <Goal {address} />
  <!-- In source order the index, the section and the tree: one column
       under 768 px reads them in that order, so a phone reaches the
       section, and a file open on the right, before the whole tree; from 768 px the index and the tree
       share the first three columns and the section spans both rows. -->
  <div class="grid grid-cols-11 grid-rows-[auto_1fr] items-start gap-x-gutter gap-y-wide narrow:grid-cols-1 narrow:grid-rows-none">
    <nav
      class="col-[1/4] row-start-1 min-w-0 narrow:col-span-full narrow:row-auto"
      aria-label={say($lang, "bld_index")}
    >
      <ul class="flex flex-col narrow:flex-row narrow:flex-wrap narrow:gap-tight">
        {#each SECTIONS as each (each)}
          <li>
            <button
              type="button"
              class={[
                "relative flex h-control w-full items-center rounded-control px-snug text-left text-note narrow:w-auto",
                section === each ? "wash-strong text-text" : "text-text-quiet hover:wash",
              ]}
              aria-current={section === each ? "true" : undefined}
              onclick={() => {
                section = each;
                picked = null;
              }}
            >
              {#if section === each}
                <span class="absolute inset-y-snug left-0 w-hair rounded-pill bg-accent" aria-hidden="true"></span>
              {/if}
              {say($lang, SECTION_WORD[each])}
            </button>
          </li>
        {/each}
      </ul>
    </nav>
    <section
      class={[
        "row-[1/3] flex min-w-0 flex-col narrow:col-span-full narrow:row-auto",
        opened === null ? "col-[4/12]" : "col-[4/8]",
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
      {:else if section === "skills"}
        <Skills building={address} onPick={pick} />
      {:else if section === "sandbox"}
        <Sandbox {address} held={building?.sandbox ?? null} known={building !== undefined} />
      {:else if picked?.kind === "file"}
        <FileView at={picked.at} root={address} />
      {:else if picked?.kind === "directory"}
        <Directory at={picked.at} root={address} onPick={pick} />
      {/if}
    </section>
    {#if opened !== null}
      <Opened item={opened} />
    {/if}
    <div class="col-[1/4] row-start-2 flex min-w-0 flex-col gap-wide narrow:col-span-full narrow:row-auto">
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
  confirmLabel={fill(say($lang, "bld_remove"), { addr: address })}
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
