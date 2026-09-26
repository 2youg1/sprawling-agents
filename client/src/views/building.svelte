<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One building, one click below the city: one information bar, the
  // directory tree, whatever the tree picked, and - where the screen
  // affords a third column - the rooms of this building with whoever is
  // working in them.
  //
  // The tree is a column from 1024 px up and a panel a button opens
  // below that, so a narrow screen keeps one column without losing the
  // way in. What a person can ask of this building - stop it, set it a
  // standing goal, take the goal away - is spelled as commands
  // (client-SPEC 4-10): the verbs a person reads are the commands they
  // would type, so the screen teaches the command line rather than a
  // private vocabulary.
  // What the tree hands back crosses a component file boundary as a
  // type. typescript-eslint resolves no named export of a `.svelte`
  // module in its type program, so the type-aware rules below see
  // `Picked` as an error type; `svelte-check` is the type gate and
  // resolves it (client-SPEC 4-1). Each suppression names that gap and
  // nothing else, the shape `parts/kbd.svelte` records its own toolchain
  // gap with.
  import type { Picked } from "./building/tree.svelte";

  // What the right-hand pane shows: the plan, the commits, or what the
  // tree picked.
  type Shown =
    | { readonly kind: "plan" }
    | { readonly kind: "commits" }
    | { readonly kind: "changes" }
    | { readonly kind: "skills" }
    // eslint-disable-next-line @typescript-eslint/no-redundant-type-constituents -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
    | Picked;

  const PLAN: Shown = { kind: "plan" };
  const COMMITS: Shown = { kind: "commits" };
  const CHANGES: Shown = { kind: "changes" };
  const SKILLS: Shown = { kind: "skills" };
</script>

<script lang="ts">
  import { readAnswer } from "../core/answered";
  import { QUERIES } from "../core/asking";
  import { halt, pursue, release, removeBuilding } from "../core/commands";
  import { fill, say } from "../core/lang";
  import { pursuitClause } from "../core/pursuit";
  import { removalOf } from "../core/removal";
  import { toFragment } from "../core/route";
  import { within } from "../core/belief/live";
  import { buildingIsShut } from "../core/scope";
  import { ui } from "../ui";
  import type { Address, Query } from "../wire";
  import Badge from "./parts/badge.svelte";
  import Button from "./parts/button.svelte";
  import Unanswered from "./parts/unanswered.svelte";
  import Dialog from "./parts/dialog.svelte";
  import Commits from "./building/commits.svelte";
  import Directory from "./building/directory.svelte";
  import FileView from "./building/file.svelte";
  import Plan from "./building/plan.svelte";
  import Skills from "./building/skills.svelte";
  import Status from "./building/status.svelte";
  import Tree from "./building/tree.svelte";
  import Rooms from "./building/rooms.svelte";

  interface Props {
    readonly address: Address;
  }

  const { address }: Props = $props();

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  let shown = $state.raw<Shown>(PLAN);
  let treeOpen = $state(false);
  let goal = $state("");
  let goalField = $state<HTMLInputElement | undefined>(undefined);

  const question = $derived<Query>({ building_view: { addr: address } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (answer) => ("building" in answer ? answer.building : undefined)));
  const building = $derived(read.kind === "held" ? read.value : undefined);

  const city = u.conn.asking.ask(QUERIES.city);
  const pursuit = $derived.by(() => {
    const held = $city;
    return held !== undefined && "city" in held
      ? held.city.pursuits.find((line) => line.addr === address)
      : undefined;
  });

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

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
  const picked = $derived(shown.kind === "file" || shown.kind === "directory" ? shown : null);

  function pick(next: Shown): void {
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment -- typescript-eslint resolves no named export of a `.svelte` module; `svelte-check` resolves it and is the type gate
    shown = next;
    treeOpen = false;
  }

  // An empty goal sends nothing; the press moves the caret to the field,
  // so the person sees where the goal is missing.
  function setGoalNow(): void {
    const words = goal.trim();
    if (words === "") {
      goalField?.focus();
    } else if (u.send(pursue(address, { set: { goal: words } }))) {
      goal = "";
    }
  }

  // How many runs are working at or below a room, which is what the
  // rooms column lights its dots for.
  function livingIn(room: Address): number {
    return $belief.live.filter((run) => within(run, room)).length;
  }
</script>

<div class="flex min-h-0 w-full flex-1 flex-col">
  <header
    class="flex flex-wrap items-center gap-base border-b border-edge px-pane py-snug"
    aria-label={say($lang, "bld_bar")}
  >
    <a href={toFragment({ kind: "city" })} class="text-note text-text-faint hover:text-text-quiet">
      {say($lang, "nav_city")}
    </a>
    <!-- wording-ok: the path separator between two addresses, hidden from readers -->
    <span class="text-text-faint" aria-hidden="true">/</span>
    <h1 class="text-title font-title" tabindex="-1">{address}</h1>
    {#if done !== null}
      <Badge
        text={fill(say($lang, "bld_progress"), {
          done: String(done.done),
          total: String(done.total),
        })}
      />
    {/if}
    <!-- wording-ok: a drawn flag marking where the standing goal lives, hidden from readers -->
    <span class="text-text-faint" aria-hidden="true">⚑</span>
    {#if pursuit === undefined}
      <!-- The field wears the same box as every other input, and its button
           is always there, so a person reads it as a place to type rather
           than as a label. -->
      <input
        class="h-control min-w-0 flex-1 rounded-control border border-edge-input bg-raised px-base text-note placeholder:text-text-faint"
        placeholder={fill(say($lang, "bld_goal_placeholder"), { addr: address })}
        bind:value={goal}
        bind:this={goalField}
        onkeydown={(event) => {
          if (event.key === "Enter") setGoalNow();
        }}
      />
      <Button
        label={say($lang, "bld_pursue")}
        tone={goal.trim() === "" ? "secondary" : "primary"}
        onPress={() => {
          setGoalNow();
        }}
      />
    {:else}
      <span
        class={[
          "inline-block size-dot rounded-pill",
          pursuit.state === "running" ? "bg-accent" : "bg-mark",
        ]}
      ></span>
      <span class="min-w-0 flex-1 truncate text-note text-text-quiet">{pursuit.goal}</span>
      <span class="font-mono text-note text-text-faint">{pursuitClause($lang, pursuit.verdict)}</span>
      <Button
        label={pursuit.state === "running" ? say($lang, "bld_pause") : say($lang, "bld_resume")}
        tone="quiet"
        onPress={() => {
          u.send(pursue(address, pursuit.state === "running" ? "pause" : "resume"));
        }}
      />
      <Button
        label={say($lang, "bld_clear")}
        tone="quiet"
        onPress={() => {
          u.send(pursue(address, "clear"));
        }}
      />
    {/if}
    <Button
      label={fill(say($lang, halted ? "bld_release" : "bld_halt"), { addr: address })}
      tone={halted ? "secondary" : "quiet"}
      onPress={() => {
        u.send(halted ? release({ building: address }) : halt({ building: address }));
      }}
    />
    <!-- Halt still shuts an idle building to new work and ends its backlog
         (glossary: Halt), so the control stays live and only says that
         nothing runs here now. -->
    {#if !halted && livingIn(address) === 0}
      <span class="text-note text-text-quiet">{say($lang, "bld_halt_idle")}</span>
    {/if}
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
    <span class="@lg/page:hidden">
      <Button
        label={say($lang, "bld_tree")}
        tone="quiet"
        onPress={() => {
          treeOpen = !treeOpen;
        }}
      />
    </span>
  </header>

  <div class="flex min-h-0 flex-1 flex-col @lg/page:flex-row">
    <!-- Below the tree's width the four pages stay on screen as one row
         that scrolls sideways, and the files toggle opens the tree with
         the rooms under it: hiding the pages behind "files" left a narrow
         screen with the plan and no visible way to the other three. -->
    <aside
      class="shrink-0 border-b border-edge px-snug py-base @lg/page:w-tree @lg/page:overflow-y-auto @lg/page:border-r @lg/page:border-b-0"
    >
      <div class="flex gap-tight overflow-x-auto @lg/page:flex-col @lg/page:gap-0 @lg/page:overflow-visible">
      <button
        type="button"
        class={[
          "flex h-step shrink-0 items-center rounded-control pl-tight pr-snug text-left text-note leading-none @lg/page:mb-tight @lg/page:w-full",
          shown.kind === "plan" ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
        ]}
        aria-current={shown.kind === "plan" ? "true" : undefined}
        onclick={() => {
          pick(PLAN);
        }}
      >
        <span class="flex w-base shrink-0 justify-center text-text-faint">≡</span>
        <span class="ml-tight">{say($lang, "bld_plan")}</span>
      </button>
      <button
        type="button"
        class={[
          "flex h-step shrink-0 items-center rounded-control pl-tight pr-snug text-left text-note leading-none @lg/page:mb-tight @lg/page:w-full",
          shown.kind === "commits" ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
        ]}
        aria-current={shown.kind === "commits" ? "true" : undefined}
        onclick={() => {
          pick(COMMITS);
        }}
      >
        <!-- wording-ok: the git branch mark naming the commits list, hidden from readers -->
        <span class="flex w-base shrink-0 justify-center font-mono text-text-faint">⎇</span>
        <span class="ml-tight">{say($lang, "bld_commits")}</span>
      </button>
      <button
        type="button"
        class={[
          "flex h-step shrink-0 items-center rounded-control pl-tight pr-snug text-left text-note leading-none @lg/page:mb-tight @lg/page:w-full",
          shown.kind === "changes" ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
        ]}
        aria-current={shown.kind === "changes" ? "true" : undefined}
        onclick={() => {
          pick(CHANGES);
        }}
      >
        <!-- wording-ok: the plus-minus mark naming the changes list, hidden from readers -->
        <span class="flex w-base shrink-0 justify-center font-mono text-text-faint">±</span>
        <span class="ml-tight">{say($lang, "bld_changes")}</span>
      </button>
      <button
        type="button"
        class={[
          "flex h-step shrink-0 items-center rounded-control pl-tight pr-snug text-left text-note leading-none @lg/page:mb-tight @lg/page:w-full",
          shown.kind === "skills" ? "bg-raised text-text" : "text-text-quiet hover:bg-chrome",
        ]}
        aria-current={shown.kind === "skills" ? "true" : undefined}
        onclick={() => {
          pick(SKILLS);
        }}
      >
        <!-- wording-ok: the asterisk mark naming the skills list, hidden from readers -->
        <span class="flex w-base shrink-0 justify-center text-text-faint">✳</span>
        <span class="ml-tight">{say($lang, "bld_skills")}</span>
      </button>
      </div>
      <div class={[treeOpen ? "mt-base block" : "hidden", "@lg/page:mt-tight @lg/page:block"]}>
        <Tree root={address} {picked} onPick={pick} />
        {#if building !== undefined}
          <div class="mt-base border-t border-edge pt-base @wide/page:hidden">
            <Rooms answer={building} living={livingIn} onPick={(room) => { pick({ at: room, kind: "directory" }); }} />
          </div>
        {/if}
      </div>
    </aside>
    <section class="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto px-pane py-base">
      {#if shown.kind === "plan"}
        {#if read.kind === "held"}
          <Plan answer={read.value} />
        {:else if read.kind === "unavailable"}
          <Unanswered query={read.query} asked={question} />
        {:else}
          <p class="text-text-faint">…</p>
        {/if}
      {:else if shown.kind === "commits"}
        <Commits building={address} />
      {:else if shown.kind === "changes"}
        <Status building={address} />
      {:else if shown.kind === "skills"}
        <Skills building={address} onPick={pick} />
      {:else if shown.kind === "file"}
        <FileView at={shown.at} root={address} />
      {:else if shown.kind === "directory"}
        <Directory at={shown.at} root={address} onPick={pick} />
      {/if}
    </section>
    <aside
      class="hidden shrink-0 overflow-y-auto border-l border-edge px-pane py-base @wide/page:block @wide/page:w-tree"
      aria-label={say($lang, "bld_rooms")}
    >
      {#if building !== undefined}
        <Rooms answer={building} living={livingIn} onPick={(room) => { pick({ at: room, kind: "directory" }); }} />
      {/if}
    </aside>
  </div>
</div>

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
