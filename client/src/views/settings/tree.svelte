<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings tree, the seat: APG Disclosure Navigation (client/Spec.lean
  // §7-11). A branch is a button, and one branch is open at a time - the
  // one the panel stands in when it opens (client D53, the fold of
  // `client/spec/Views/Fold.lean`, moved by `folds.ts`); a group entry
  // is a button that draws its group beside the tree; a page entry is a
  // link, so a middle click and a new tab work; a nest and a branch's
  // "more" are buttons that open their own list. ↓/↑ walk the entries a
  // person can see, Home/End jump to the ends, and Tab still steps
  // through them one by one; the keys are the line keys of
  // `core/lines.ts`. A letter here is a first letter: it moves to the
  // next group that starts with it, wrapping at the end (client D40), so
  // j, k, g and G do not walk this tree.
  //
  // The performance entry carries the city's process reading in one
  // line (docs/frontend-method.md §7D): while the tree is drawn this page
  // asks the monitor for its summary, and the city samples only while
  // somebody watches.
  //
  // This file owns the folds, the keys and the focus; `lookOf`
  // (`tree.ts`) builds every word, state and wire bag, and the look
  // (`tree.look.svelte`) draws them.

  import { onDestroy } from "svelte";

  import { initialTyped, lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { summary } from "../../core/monitor";
  import { LENSES } from "../../core/route";
  import type { SetupGroup, View } from "../../core/route";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { useBuildings } from "../shared/buildings";
  import { foldsOf, toggled } from "./folds";
  import type { Folds } from "./folds";
  import { buildingPages, byInitial, lookOf, stepped } from "./tree";
  import type { Nest, Page } from "./tree";
  import Look from "./tree.look.svelte";

  interface Props {
    // The group drawn in the body now.
    readonly group: SetupGroup;
    // The page under the panel: its entry says so, and its nest is open.
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
  }

  const { group, beneath, onPick }: Props = $props();

  const u = ui();
  const { lang } = u;
  const buildings = useBuildings();
  const samples = u.conn.monitor.samples;
  onDestroy(u.conn.monitor.watchSummary());
  const reading = $derived.by(() => {
    const latest = $samples.at(-1);
    if (latest === undefined) return null;
    const read = summary(latest);
    return `${read.cpu} · ${read.memory}`;
  });

  const uid = $props.id();
  // svelte-ignore state_referenced_locally (the folds the panel opened on start open; the person folds them after)
  let folds = $state.raw<Folds>(foldsOf(group, beneath));

  // The pages under each nest, read when the nest is drawn.
  const leaves = $derived<Record<Nest, readonly Page[]>>({
    buildings: buildingPages($buildings),
    record: LENSES.map((lens) => ({ kind: "page", view: { kind: "record", lens }, word: `rec_${lens}` })),
  });

  let nav: HTMLElement | null = null;
  const hold = (node: HTMLElement): (() => void) => {
    nav = node;
    return () => {
      nav = null;
    };
  };

  const lines = lineWalker();

  function walk(event: KeyboardEvent): void {
    if (nav === null) return;
    const pressed = pressedOf(event);
    const entries = [...nav.querySelectorAll<HTMLElement>("[data-entry]")];
    const at = entries.findIndex((each) => each === document.activeElement);
    const letter = initialTyped(pressed);
    const next =
      letter === null
        ? stepped(entries, at, lines(pressed, event.timeStamp))
        : byInitial(entries, at, letter, (each) => each.dataset.initial);
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  const look = $derived(
    lookOf({ group, beneath, folds, leaves, reading }, uid, {
      words: (key) => say($lang, key),
      pick: onPick,
      toggle: (fold) => {
        folds = toggled(folds, fold);
      },
      walk,
      hold,
    }),
  );
</script>

<Look {...look} />
