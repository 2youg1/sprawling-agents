<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One answer about this machine, drawn. Separate from the ask so the
// gallery can show this machine in states nobody's own machine happens
// to be in.
//
// **A card per program, and a grid as wide as the body.** A card
// carries its own width: name, state, version, what the program
// enables, the command, and the two controls at one height. The grid
// asks for a minimum column of 320 points rather than a breakpoint,
// because the column width is a fact about the card and the breakpoint
// would be a guess about the window.
//
// The two tiers stay, because a person acts differently on them: what
// the city cannot run without, and what it would use if the machine had
// it. Each tier keeps its own count and its own verdict, and the city's
// verdict is not the rows below it (see `standing` in
// `views/setup/dependencies`).
//
// One click installs. `Command::DoctorInstall` runs the recipe this
// city may run and refuses the other two with what a person does
// instead, and the install ends by looking at this machine again - so
// `check again` is the manual half of the same verb rather than a
// second way of getting the same answer.

import type { Key } from "../../core/lang";
import type { DoctorItem } from "../../wire";
import type { Weight } from "../parts/glyph";
import type { Tone } from "../parts/button.svelte";
import type { Offer } from "../setup/dependencies";

// What the badge beside the name weighs. The word is what states the
// answer; the weight only decides how loudly, and a broken or required
// missing item is the one a person has to act on.
function weightOf(item: DoctorItem): Weight {
  if ("present" in item.state) return "quiet";
  if ("broken" in item.state) return "alert";
  return item.need === "optional" ? "quiet" : "alert";
}

// The paint each offer carries, and the reason it carries when a press
// does nothing. A reason is what makes the control grey - `Button`
// owns that rule - so this table states the reason and never the grey.
//
// **No offer takes the primary tone** (ux-upgrades A11): install is a
// per-card act, and a row's act is not the one button a screen is for.
// The screen's single primary is `check again`, above the cards.
const OFFER: Record<Offer, { readonly tone: Tone; readonly why: Key | null }> = {
  press: { tone: "secondary", why: null },
  by_hand: { tone: "quiet", why: "machine_install_by_hand" },
  held: { tone: "quiet", why: "machine_installed" },
};
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import type { DoctorAnswer } from "../../wire";
  import { ui } from "../../ui";
  import { offerOf, outstanding, recommended, required, spelledOf, standing, stateKey, versionOf } from "../setup/dependencies";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Progress from "../parts/progress.svelte";
  import Tip from "../parts/tip.svelte";
  import Copy from "./copy.svelte";

  interface Props {
    readonly answer: DoctorAnswer;
    readonly onInstall?: (item: string) => void;
  }

  const { answer, onInstall }: Props = $props();

  const { lang } = ui();

  const first = $derived(required(answer));
  const second = $derived(recommended(answer));
  const missedUse = $derived(outstanding(answer, "use"));
  const missedDevelop = $derived(outstanding(answer, "develop"));

  // One program, and everything a person decides about it from one
  // card: the name, the state, the version, what it enables, and the
  // command that would get it. The name links to the program's own
  // site when the city knows one, so a person can read what a thing is
  // before installing it.
</script>

{#snippet card(item: DoctorItem)}
  {@const said = versionOf(item.state)}
  {@const how = spelledOf(item.install)}
  {@const offer = OFFER[offerOf(item)]}
  <li class="flex min-w-0 flex-col gap-snug rounded-card bg-raised p-base">
    <div class="flex min-w-0 flex-wrap items-center gap-snug">
      {#if item.homepage === undefined || item.homepage === null}
        <span class="font-mono text-label text-text">{item.name}</span>
      {:else}
        {@const site = item.homepage}
        <Tip text={site}>
          {#snippet children(hint)}
            <a
              class="font-mono text-label text-text underline decoration-edge underline-offset-2 hover:decoration-accent"
              href={site}
              target="_blank"
              rel="noreferrer"
              aria-describedby={hint}
            >
              {item.name}
            </a>
          {/snippet}
        </Tip>
      {/if}
      <Badge text={say($lang, stateKey(item.state))} weight={weightOf(item)} dot />
      {#if item.need === "optional"}
        <span class="text-note text-text-faint">{say($lang, "machine_optional")}</span>
      {/if}
      <span class="min-w-0 flex-1"></span>
      {#if said !== null}
        <span class="min-w-0 truncate text-note text-text-quiet">{said}</span>
      {/if}
    </div>
    <p class="text-note text-text-faint">{item.enables}</p>
    {#if how === null}
      <span class="text-note text-text-faint">{say($lang, "machine_no_recipe")}</span>
    {:else}
      <div class="flex min-w-0 flex-col gap-snug">
        <!-- A command cut at the card's edge cannot be typed, and a
        command pushed sideways has to be scrolled before it can be
        read whole: pre-wrap keeps every character on the card. Each
        word is one box, so a line breaks between words and never at
        the hyphen inside `--locked`; only a word wider than the card
        breaks inside itself. -->
        <code class="block whitespace-pre-wrap break-words rounded-control bg-chrome px-snug py-tight font-mono text-note text-text-quiet"
          >{#each how.split(" ") as word, index (index)}{index > 0 ? " " : ""}<span class="inline-block max-w-full"
              >{word}</span
            >{/each}</code
        >
        <div class="flex flex-wrap items-center gap-tight">
          <Button
            label={say($lang, "machine_install")}
            tone={offer.tone}
            {...(offer.why === null ? {} : { why: say($lang, offer.why) })}
            onPress={() => {
              onInstall?.(item.name);
            }}
          />
          <Copy text={how} />
        </div>
      </div>
    {/if}
  </li>
{/snippet}

{#snippet column(title: string, missing: readonly string[] | null, items: readonly DoctorItem[])}
  {@const far = standing(items, missing === null ? null : missing.length)}
  <section class="flex min-w-0 flex-col gap-snug" aria-label={title}>
    <h2 class="text-label font-label text-text-quiet">{title}</h2>
    <Progress label={say($lang, "machine_progress_label")} done={far.done} total={far.total} />
    <!-- The city's own verdict on this tier, which is not the cards
    below it: a run of interchangeable items collapses into the one
    thing a person is still missing, so a tier of empty cards can be a
    tier with nothing left to do. The names are set apart by the gap
    between them rather than joined by a word, because a conjunction
    would be a second authority on how a list reads in each language. -->
    {#if missing !== null && missing.length > 0}
      <p class="flex flex-wrap items-baseline gap-tight text-note">
        <span class="text-text-quiet">{say($lang, "machine_missing")}</span>
        {#each missing as name (name)}
          <span class="font-mono text-alert">{name}</span>
        {/each}
      </p>
    {/if}
    <ul class="grid grid-cols-[repeat(auto-fit,minmax(320px,1fr))] gap-base">
      {#each items as item (item.name)}
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render card(item)}
      {/each}
    </ul>
  </section>
{/snippet}

<div class="flex min-w-0 flex-col gap-wide">
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render column(say($lang, "machine_required"), missedUse, first)}
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render column(say($lang, "machine_recommended"), missedDevelop, second)}
</div>
