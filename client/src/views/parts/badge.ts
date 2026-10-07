// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A small fact attached to something larger: how many are waiting, or
// where a thing stands (client/Spec.lean §4-32). The word is always
// drawn, and the mark and the colour only repeat it - a state told by
// colour alone is a state that is not told to everybody.
//
// Three ways to be a badge, and a badge takes exactly one. `status`
// takes both its drawing and its paint from the one status table in
// `glyph.ts`, so a chip here and a dot elsewhere tell the same story;
// `weight` and `dot` are the plain form for facts that have no state
// vocabulary; `count` is the number pinned to the corner of a key,
// which repeats what the key's own name already says. The union is
// what makes "a status painted at a caller's tier" unrepresentable: two
// authorities for one paint cannot meet in one badge.

import { statusLook } from "./glyph";
import type { GlyphName, Status, Weight } from "./glyph";

export interface BadgeProps {
  // Already in the person's language, or a number the caller formatted.
  readonly text: string;
  readonly status?: Status;
  readonly weight?: undefined;
  readonly dot?: undefined;
  readonly count?: undefined;
}

export interface BadgeTierProps {
  readonly text: string;
  readonly status?: undefined;
  // Nothing in particular, something going on, something wrong. The
  // three tiers of client/Spec.lean §4-32.
  readonly weight?: Weight;
  // A state reads better with a mark beside it; a count does not.
  readonly dot?: boolean;
  readonly count?: undefined;
}

export interface BadgeCountProps {
  // How many wait. Zero draws nothing; `"fresh"` draws a lone dot for
  // something new that has no number.
  readonly count: number | "fresh";
  // The largest number drawn as itself; past it the badge reads
  // `cap+`, so a corner mark never grows wider than the key it marks.
  readonly cap?: number;
  readonly weight?: Weight;
  readonly text?: undefined;
  readonly status?: undefined;
  readonly dot?: undefined;
}

export type AnyBadgeProps = BadgeProps | BadgeTierProps | BadgeCountProps;

export const CAP = 99;

// What a look of a badge is given. A word badge is read aloud; a count
// is hidden from a screen reader (`quiet` spread on it) because the key
// it is pinned to names the same number.
export type BadgeLook =
  | {
      readonly form: "word";
      readonly text: string;
      readonly mark: GlyphName | "dot" | undefined;
      readonly tier: Weight;
    }
  | {
      readonly form: "count";
      // The number as drawn, or `undefined` for the lone dot.
      readonly text: string | undefined;
      readonly tier: Weight;
      readonly quiet: Quiet;
    };

export interface Quiet {
  readonly "aria-hidden": "true";
}

// The one reading of what a badge is marked with, or `undefined` when a
// count of zero draws nothing.
export function badgeOf(props: AnyBadgeProps): BadgeLook | undefined {
  if (props.count !== undefined) return countOf(props.count, props.cap ?? CAP, props.weight ?? "alert");
  if (props.status !== undefined) {
    const look = statusLook(props.status);
    return { form: "word", text: props.text, mark: look.glyph, tier: look.weight };
  }
  return { form: "word", text: props.text, mark: props.dot === true ? "dot" : undefined, tier: props.weight ?? "quiet" };
}

function countOf(count: number | "fresh", cap: number, tier: Weight): BadgeLook | undefined {
  const quiet: Quiet = { "aria-hidden": "true" };
  if (count === "fresh") return { form: "count", text: undefined, tier, quiet };
  if (count <= 0) return undefined;
  return { form: "count", text: count > cap ? `${String(cap)}+` : String(count), tier, quiet };
}
