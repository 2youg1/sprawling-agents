<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Several readings of one subject, one at a time: the lenses of a run,
  // of the record, of the cost page.
  //
  // This is the seat: it holds the drawn tabs so the focus can follow an
  // arrow, measures whether the shown panel holds anything the Tab key
  // reaches, and draws whatever `./tabs.look.svelte` is. Every role, id,
  // key and association is decided in `./tabs` (`lookOf`), so a look -
  // this one or one built from a component library - only spreads the
  // wire bags it is handed (client/spec/Views/Parts.lean §7-4).
  //
  // One stop on the way in and the arrows once inside - the roving
  // tabindex ARIA asks for - so a keyboard crossing the page does not
  // have to step through every lens to get past them. Activation is
  // automatic, the APG reading for panels whose content is already
  // local and whose switch has no perceptible delay: focus moves and
  // the panel switches together, and Space and Enter take the same
  // path as a click without needing a line of their own.
  //
  // **The tab and its panel are one fact, so this part owns both halves
  // of the association.** The caller attaches each reading through the
  // `panel` snippet and an optional `mark` beside a tab's name; the part
  // draws the panel inside the wrapper that carries `role="tabpanel"`
  // and `aria-labelledby`. The contract:
  //
  // - Every tab carries a unique `id` and an `aria-controls` pointing
  //   at its panel; ids are prefixed per instance (`$props.id()`), so
  //   two tab sets on one page never claim each other's panels.
  // - A wrapper exists for every lens, shown or not, so `aria-controls`
  //   always resolves; the panel content itself mounts while its lens
  //   is current and unmounts when the lens leaves, so a caller may use
  //   mount as its data lifecycle and never pays for a lens nobody is
  //   reading.
  // - The shown panel is a Tab stop of its own while nothing inside it
  //   takes the focus, so a panel of plain text is still reached and
  //   read; once it holds a control, that control is the next stop.
  // - The panels render as siblings after the tablist. A caller that
  //   wants one column wraps this part; a caller that seats the tablist
  //   beside other furniture keeps that row around the tablist and moves
  //   the panels under it.
  // - `panel` receives the lens being shown, so one exhaustive
  //   if-chain over `lens.id` names every reading and a new lens cannot
  //   fall through to a wrong panel.

  export type { Lens, TabsProps } from "./tabs";
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";

  import { lookOf, REACHABLE } from "./tabs";
  import type { TabsProps } from "./tabs";
  import Look from "./tabs.look.svelte";

  const props: TabsProps = $props();
  const uid = $props.id();

  // Two registries no draw reads, so plain records rather than reactive
  // maps: a reactive map read inside the derived look and written by the
  // attachment it hands out would be a write during a derivation. Keyed
  // by lens id, so an element stays with its lens when the set is edited
  // rather than with the position it happened to sit at.
  const drawn: Record<string, HTMLElement | undefined> = {};
  const kept: Record<string, Attachment<HTMLElement> | undefined> = {};

  const keep = (id: string): Attachment<HTMLElement> => {
    const held = kept[id];
    if (held !== undefined) return held;
    const made: Attachment<HTMLElement> = (node) => {
      drawn[id] = node;
      return () => {
        if (drawn[id] === node) drawn[id] = undefined;
      };
    };
    kept[id] = made;
    return made;
  };

  // Whether the shown panel holds something the Tab key reaches. A
  // panel's content arrives when its data does, so the panel is watched
  // while it is shown and measured at most once a frame.
  let reachable = $state(true);

  const watch: Attachment<HTMLElement> = (node) => {
    let frame = 0;
    const measure = (): void => {
      frame = 0;
      reachable = node.querySelector(REACHABLE) !== null;
    };
    const observer = new MutationObserver(() => {
      if (frame === 0) frame = requestAnimationFrame(measure);
    });
    observer.observe(node, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["disabled", "href", "tabindex", "contenteditable", "type"],
    });
    measure();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  };

  const look = $derived(
    lookOf(props, uid, {
      focus: (id) => drawn[id]?.focus(),
      keep,
      watch,
      reachable,
    }),
  );
</script>

<Look {...look} />
