// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One hint, drawn beside the control it is about.
//
// This replaces `title`, which fails three readers: it waits about a
// second for a pointer a keyboard never has, no key reaches it, and a
// touch screen never draws it at all. The hint here appears on hover
// and on focus anywhere inside the wrapper, so tabbing to the control
// shows it.
//
// **The caller states the relation, because only the caller knows
// whether the control has a name already.** The child is handed the
// hint's id and writes `aria-describedby` when it is named by its own
// text, or `aria-labelledby` when these words are the only name it has.
// Writing neither leaves a hint a screen reader never reads.
//
// **Placement has two branches, because engines disagree.** Where
// anchor positioning is implemented the hint is `fixed` against the
// anchor and leaves every box that clips behind it. Where it is not -
// Safari and Firefox today - the hint is placed against the wrapper,
// which is why the wrapper is `relative`. Shipping only the first
// branch fails silently: the hint keeps a static position and can land
// outside the window.
//
// The hint is `display: none` until it is wanted, so a screen holding
// fifty controls draws fifty nothings; the 300 ms delay is what stops a
// pointer crossing a row from lighting them one after another.

import { createUniqueId } from "solid-js";
import type { JSX } from "solid-js";

// Against the wrapper, for an engine without anchor positioning.
const AGAINST_WRAPPER = "absolute bottom-full left-1/2 -translate-x-1/2";

// Against the anchor, for an engine with it. `flip-block` drops the
// hint below the control when there is no room above.
//
// `--tip-anchor` is spelled out rather than built from a constant
// because Tailwind reads class names out of this file as text: a name
// assembled at run time produces no CSS at all.
const AGAINST_ANCHOR =
  "supports-[anchor-name:--a]:fixed supports-[anchor-name:--a]:bottom-auto " +
  "supports-[anchor-name:--a]:left-auto supports-[anchor-name:--a]:translate-x-0 " +
  "supports-[anchor-name:--a]:[position-anchor:var(--tip-anchor)] " +
  "supports-[anchor-name:--a]:[position-area:block-start] " +
  "supports-[anchor-name:--a]:[position-try-fallbacks:flip-block]";

// Hidden costs nothing to draw and nothing to measure; shown is what
// hover and focus both switch to.
const WHEN_WANTED =
  "hidden opacity-0 group-hover/tip:block group-hover/tip:opacity-100 " +
  "group-focus-within/tip:block group-focus-within/tip:opacity-100";

const PAINT =
  "pointer-events-none z-30 mb-tight w-max max-w-measure rounded-card border border-g3 bg-g2 " +
  "px-snug py-tight text-note text-text shadow-composer " +
  "transition-[opacity,display] transition-discrete delay-300 duration-200 ease-standard " +
  "motion-reduce:transition-none";

export interface TipProps {
  // Already in the person's language.
  readonly text: string;
  // The control the hint is about, handed the hint's id: as
  // `aria-describedby` when the control is named by its own text, as
  // `aria-labelledby` when these words are that name.
  readonly children: (hint: string) => JSX.Element;
}

export function Tip(props: TipProps) {
  const hint = createUniqueId();
  // One anchor name per instance, carried to the hint by inheritance, so
  // two hints on one row anchor to their own control rather than both to
  // whichever came last in the document.
  return (
    <span
      class="group/tip relative inline-flex min-w-0 items-center [anchor-name:var(--tip-anchor)]"
      style={{ "--tip-anchor": `--tip-${hint}` }}
    >
      {props.children(hint)}
      <span id={hint} role="tooltip" class={`${PAINT} ${WHEN_WANTED} ${AGAINST_WRAPPER} ${AGAINST_ANCHOR}`}>
        {props.text}
      </span>
    </span>
  );
}
