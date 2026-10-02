<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // Several readings of one subject, one at a time: the lenses of a run,
  // of the record, of the cost page.
  //
  // One stop on the way in and the arrows once inside - the roving
  // tabindex ARIA asks for - so a keyboard crossing the page does not
  // have to step through every lens to get past them. Activation is
  // automatic, the APG reading for panels whose content is already
  // local and whose switch has no perceptible delay: focus moves and
  // the panel switches together, and Space and Enter take the same
  // path as a click without needing a line of their own.
  //
  // **The tab and its panel are one fact, so this component owns both
  // halves of the association (client/Spec.lean §7-8 item 6).** The caller
  // attaches each reading through the `panel` snippet and an optional
  // `mark` beside a tab's name; this component draws the panel inside
  // the wrapper that carries `role="tabpanel"` and `aria-labelledby`.
  // The contract:
  //
  // - Every tab carries a unique `id` and an `aria-controls` pointing
  //   at its panel; ids are prefixed per instance (`$props.id()`), so
  //   two tab sets on one page never claim each other's panels.
  // - A wrapper exists for every lens, shown or not, so `aria-controls`
  //   always resolves; the panel content itself mounts while its lens
  //   is current and unmounts when the lens leaves, so a caller may use
  //   mount as its data lifecycle and never pays for a lens nobody is
  //   reading.
  // - The panels render as siblings after the tablist. A caller that
  //   wants one column wraps this component; a caller that seats the
  //   tablist beside other furniture keeps that row around the tablist
  //   and moves the panels under it.
  // - `panel` receives the lens being shown, so one exhaustive
  //   if-chain over `lens.id` names every reading and a new lens cannot
  //   fall through to a wrong panel.

  import type { Snippet } from "svelte";

  // One lens of the subject. Content does not travel in this row: it
  // arrives as the `panel` and `mark` snippets, because a snippet is
  // written in markup while a row like this is built in script.
  export interface Lens {
    readonly id: string;
    // Already in the person's language.
    readonly label: string;
  }

  export interface TabsProps {
    // The accessible name of the set, already in the person's language.
    readonly label: string;
    readonly lenses: readonly Lens[];
    // The one lens shown: an id from `lenses`.
    readonly current: string;
    readonly onPick: (id: string) => void;
    // What each lens shows, drawn inside this component's
    // `role="tabpanel"` wrapper and labelled by that lens's tab.
    readonly panel: Snippet<[Lens]>;
    // A count or a state beside a tab's name, usually a Badge. The
    // snippet is asked for every tab and answers for the ones it has
    // something to say about.
    readonly mark?: Snippet<[Lens]>;
  }

  const uid = $props.id();

  const { label, lenses, current, onPick, panel, mark }: TabsProps = $props();

  // The drawn tabs, keyed by lens id, so `bind:this` keeps each element
  // with its lens when the set is edited rather than with the position
  // it happened to sit at.
  const tabs: Record<string, HTMLButtonElement | undefined> = {};

  const move = (to: number): void => {
    const lens = lenses[to];
    if (lens === undefined) return;
    onPick(lens.id);
    tabs[lens.id]?.focus();
  };

  const travel = (event: KeyboardEvent): void => {
    const last = lenses.length - 1;
    const at = lenses.findIndex((lens) => lens.id === current);
    if (event.key === "ArrowRight") {
      event.preventDefault();
      move(at === last ? 0 : at + 1);
      return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      move(at <= 0 ? last : at - 1);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      move(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      move(last);
    }
  };

  // The bar beside the current tab is the one accent this control
  // takes (docs/frontend-method.md §7B); every other tab is said by its own ink.
  const ink = (lens: Lens): string =>
    lens.id === current
      ? "border-accent text-text"
      : "border-transparent text-text-quiet hover:text-text";
</script>

<div class="flex items-center gap-tight border-b border-edge" role="tablist" aria-label={label}>
  {#each lenses as lens (lens.id)}
    <button
      bind:this={tabs[lens.id]}
      type="button"
      role="tab"
      id="{uid}-tab-{lens.id}"
      aria-controls="{uid}-panel-{lens.id}"
      aria-selected={lens.id === current}
      tabindex={lens.id === current ? 0 : -1}
      class={["flex items-center gap-tight border-b-2 px-base py-snug text-label", ink(lens)]}
      onclick={() => {
        onPick(lens.id);
      }}
      onkeydown={travel}
    >
      {lens.label}
      {@render mark?.(lens)}
    </button>
  {/each}
</div>
{#each lenses as lens (lens.id)}
  <div
    role="tabpanel"
    id="{uid}-panel-{lens.id}"
    aria-labelledby="{uid}-tab-{lens.id}"
    hidden={lens.id !== current}
  >
    {#if lens.id === current}
      {@render panel(lens)}
    {/if}
  </div>
{/each}
