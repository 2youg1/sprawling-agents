<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One answer about this machine, drawn. Separate from the ask so the
// gallery can show this machine in states nobody's own machine happens
// to be in.
//
// **One section per tier, because the city judges each tier on its
// own.** Running a city and developing sprawling are different people
// with different lists; the progress bar and the "still missing" line of
// a section are the city's verdict on that tier and nothing else (see
// `standing` in `views/setup/dependencies`).
//
// **A row per program, and a row only as tall as what is left to do.**
// A program this machine has is one line: its name, its state, its
// version (`rowOf`). A state the badge alone does not explain
// - a program that gave no version, a fault, where the city looked - adds
// one line of reason. A missing one adds the command that gets it, the install or
// copy control, and one line saying where the command gets it from and
// why the page waits for a press before running it.
//
// Under each name, the three versions (`versions.svelte`): installed
// here, pinned by this repository, newest upstream. The cargo tools this
// repository calls are one row, the Rust tools pack (`pack.svelte`),
// with one press for the members it is missing.
//
// Last, the scan in front of the city's directory (`scanning.svelte`): read
// on Windows, and on macOS and Linux the reason there is nothing to read.
//
// The develop section carries the one press that installs everything
// it is missing (`views/setup/installing`); the screen's `machine.svelte`
// sends the installs, and this file only draws how far they have got.

import type { Key } from "../../core/lang";
import type { DoctorCore, DoctorItem } from "../../wire";
import type { Weight } from "../parts/glyph";
import type { Absence } from "../setup/dependencies";
import type { StepState } from "../setup/installing";

// What the badge beside the name weighs. The word is what states the
// answer; the weight only decides how loudly, and a broken or required
// missing item is the one a person has to act on - unless it is a spare
// of a group another member already answers.
function weightOf(item: DoctorItem, absence: Absence): Weight {
  if ("present" in item.state) return "quiet";
  if ("broken" in item.state) return "alert";
  return item.need === "optional" || absence === "spare" ? "quiet" : "alert";
}

// The level the core's threads stand at, and the platform's own words
// when it refused: those are the platform's to choose, not the page's.
function coreOf(core: DoctorCore): { readonly key: Key; readonly said: string | null } {
  if (typeof core !== "string") {
    return "refused" in core
      ? { key: "machine_core_refused", said: core.refused.said }
      : { key: "machine_core_unasked", said: core.unasked.said };
  }
  switch (core) {
    case "raised":
      return { key: "machine_core_raised", said: null };
    case "held_by_setting":
      return { key: "machine_core_held", said: null };
    case "lowered_by_valve":
      return { key: "machine_core_lowered", said: null };
  }
}

const STEP: Record<StepState, { readonly key: Key; readonly weight: Weight }> = {
  waiting: { key: "machine_step_waiting", weight: "quiet" },
  running: { key: "machine_step_running", weight: "quiet" },
  done: { key: "machine_step_done", weight: "quiet" },
  failed: { key: "machine_step_failed", weight: "alert" },
};
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import type { DoctorAnswer, DoctorNewest, DoctorTier } from "../../wire";
  import { ui } from "../../ui";
  import { absenceOf, enablesKey, ofTier, outstanding, rowOf, siteOf, sourceOf, standing } from "../setup/dependencies";
  import { over, type Walk } from "../setup/installing";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Progress from "../parts/progress.svelte";
  import Tip from "../parts/tip.svelte";
  import Copy from "./copy.svelte";
  import Pack from "./pack.svelte";
  import Scanning from "./scanning.svelte";
  import Versions from "./versions.svelte";

  interface Props {
    readonly answer: DoctorAnswer;
    readonly onInstall?: (item: string) => void;
    // What one press would install, and the press itself.
    readonly planned?: readonly string[];
    readonly onInstallAll?: () => void;
    // How far that press has got, once it was pressed.
    readonly walk?: Walk | null;
    // The newest release of each item, by name, as the answers arrive.
    readonly newest?: Readonly<Record<string, DoctorNewest>>;
    // The pack's one press: its missing members, in table order.
    readonly onInstallPack?: (names: readonly string[]) => void;
  }

  const { answer, onInstall, planned = [], onInstallAll, walk = null, newest = {}, onInstallPack }: Props = $props();

  const { lang } = ui();

  const core = $derived(coreOf(answer.core));
  const walking = $derived(walk !== null && !over(walk));

  // Where the command gets the program from and why the page waits,
  // or why the page will not run it at all; a manual line says what to
  // do by itself.
  function noteOf(item: DoctorItem, how: string): string | null {
    const install = item.install;
    if (typeof install === "string" || "manual" in install) return null;
    if ("command" in install) {
      const source = sourceOf(how);
      return fill(say($lang, "machine_install_press"), {
        source: "key" in source ? say($lang, source.key) : source.program,
      });
    }
    const site = siteOf(how);
    return site === null ? say($lang, "machine_install_admin") : fill(say($lang, "machine_install_by_hand"), { site });
  }
</script>

{#snippet row(item: DoctorItem, absence: Absence)}
  {@const line = rowOf(item)}
  {@const how = line.install}
  {@const enables = enablesKey(item.name)}
  <li class="flex min-w-0 flex-col gap-tight border-t border-edge pt-snug pb-base">
    <div class="flex min-w-0 items-center gap-snug">
      {#if item.homepage === undefined || item.homepage === null}
        <span class="shrink-0 whitespace-nowrap font-mono text-label text-text">{item.name}</span>
      {:else}
        {@const site = item.homepage}
        <span class="shrink-0 whitespace-nowrap"><Tip text={site}>
          {#snippet children(hint)}
            <a
              class="font-mono text-label text-text underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
              href={site}
              target="_blank"
              rel="noreferrer"
              aria-describedby={hint}
            >
              {item.name}
            </a>
          {/snippet}
        </Tip></span>
      {/if}
      <Badge text={say($lang, line.state)} weight={weightOf(item, absence)} dot />
      {#if item.need === "optional"}
        <span class="shrink-0 text-note text-text-faint">{say($lang, "machine_optional")}</span>
      {:else if absence === "spare"}
        <span class="min-w-0 truncate text-note text-text-faint">{say($lang, "machine_spare")}</span>
      {/if}
    </div>
    <Versions {item} newest={newest[item.name]} />
    {#if line.reason !== null}
      <p class="flex min-w-0 flex-wrap items-baseline gap-tight text-note">
        <span class="text-text-quiet">{say($lang, line.reason.key)}</span>
        {#if line.reason.said !== null}
          <span class="min-w-0 break-all font-mono text-text-faint">{line.reason.said}</span>
        {/if}
      </p>
    {/if}
    {#if enables !== null}
      <p class="text-note text-text-faint">{say($lang, enables)}</p>
    {/if}
    {#if line.offer !== "held"}
      {#if how === null}
        <span class="text-note text-text-faint">{say($lang, "machine_no_recipe")}</span>
      {:else}
        {@const note = noteOf(item, how)}
        <div class="flex min-w-0 items-start gap-tight">
          <!-- A command cut at the row's edge cannot be typed: pre-wrap
          keeps every character in the row, and a line breaks between
          words, never at the hyphen inside `--locked`. -->
          <code class="min-w-0 flex-1 whitespace-pre-wrap break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet"
            >{#each how.split(" ") as word, index (index)}{index > 0 ? " " : ""}<span class="inline-block max-w-full"
                >{word}</span
              >{/each}</code
          >
          {#if line.offer === "press"}
            <Button
              label={say($lang, "machine_install")}
              tone="secondary"
              loading={walk?.some((each) => each.name === item.name && each.state === "running") === true}
              onPress={() => {
                onInstall?.(item.name);
              }}
            />
          {/if}
          <Copy text={how} />
        </div>
        {#if note !== null}
          <p class="text-note text-text-faint">{note}</p>
        {/if}
      {/if}
    {/if}
  </li>
{/snippet}

{#snippet steps(each: Walk)}
  {@const done = each.filter((step) => step.state === "done" || step.state === "failed").length}
  <div class="flex min-w-0 flex-col gap-snug border-y border-edge py-snug">
    <!-- This bar counts the walk's steps, not the tier's items, so it is
    named on the page: unnamed, it read as a second copy of the tier's. -->
    <span class="text-note text-text-quiet">{say($lang, "machine_walk_progress")}</span>
    <Progress label={say($lang, "machine_walk_progress")} {done} total={each.length} />
    <ol class="flex min-w-0 flex-col gap-tight">
      {#each each as step (step.name)}
        <li class="flex min-w-0 flex-wrap items-baseline gap-snug text-note">
          <span class="w-[10rem] shrink-0 truncate font-mono text-text">{step.name}</span>
          <Badge text={say($lang, STEP[step.state].key)} weight={STEP[step.state].weight} dot />
          {#if step.why !== null}
            <span class="min-w-0 flex-1 break-words text-text-faint">{step.why}</span>
          {/if}
        </li>
      {/each}
    </ol>
  </div>
{/snippet}

{#snippet tier(title: string, which: DoctorTier)}
  {@const items = ofTier(answer, which)}
  {@const packed = items.filter((each) => each.pack === "rust_tools")}
  {@const missing = outstanding(answer, which)}
  {@const far = standing(items, missing === null ? null : missing.length)}
  <section class="flex min-w-0 flex-col gap-snug" aria-label={title}>
    <div class="flex min-w-0 flex-wrap items-center gap-base">
      <h2 class="text-label font-label text-text">{title}</h2>
      {#if which === "develop" && planned.length > 0 && onInstallAll !== undefined}
        <Button
          label={fill(say($lang, "machine_install_all"), { count: String(planned.length) })}
          tone="primary"
          loading={walking}
          onPress={onInstallAll}
        />
        <span class="text-note text-text-faint">{say($lang, "machine_install_all_note")}</span>
      {/if}
    </div>
    <Progress label={say($lang, "machine_progress_label")} done={far.done} total={far.total} />
    <!-- The city's own verdict on this tier, which is not the rows below
    it: a run of interchangeable items collapses into the one thing a
    person is still missing. The names are set apart by the gap between
    them rather than joined by a word, because a conjunction would be a
    second authority on how a list reads in each language. -->
    {#if missing !== null && missing.length > 0}
      <p class="flex flex-wrap items-baseline gap-tight text-note">
        <span class="text-text-quiet">{say($lang, "machine_missing")}</span>
        {#each missing as name (name)}
          <span class="font-mono text-alert">{name}</span>
        {/each}
      </p>
    {/if}
    {#if which === "develop" && walk !== null}
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
      {@render steps(walk)}
    {/if}
    <ul class="grid grid-cols-[repeat(auto-fill,minmax(340px,1fr))] items-start gap-x-gutter">
      {#each items.filter((each) => each.pack === undefined || each.pack === null) as item (item.name)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render row(item, absenceOf(item, missing, items))}
      {/each}
      {#if packed.length > 0}
        <Pack members={packed} {newest} {walking} onInstall={onInstallPack} />
      {/if}
    </ul>
  </section>
{/snippet}

<div class="flex min-w-0 flex-col gap-wide">
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render tier(say($lang, "machine_tier_use"), "use")}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render tier(say($lang, "machine_tier_develop"), "develop")}
  <p class="flex min-w-0 flex-wrap items-baseline gap-tight text-note">
    <span class="text-text-quiet">{say($lang, "machine_core")}</span>
    <span class="text-text">{say($lang, core.key)}</span>
    {#if core.said !== null}
      <span class="min-w-0 break-words text-text-faint">{core.said}</span>
    {/if}
  </p>
  <Scanning scanning={answer.scanning} />
</div>
