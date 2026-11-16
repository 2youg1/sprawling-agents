// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { statusLook } from "./glyph";
import type { GlyphName, Status, Weight } from "./glyph";

export interface BadgeProps {
  readonly text: string;
  readonly status?: Status;
  readonly weight?: undefined;
  readonly dot?: undefined;
  readonly count?: undefined;
}

export interface BadgeTierProps {
  readonly text: string;
  readonly status?: undefined;

  readonly weight?: Weight;

  readonly dot?: boolean;
  readonly count?: undefined;
}

export interface BadgeCountProps {
  readonly count: number | "fresh";

  readonly cap?: number;
  readonly weight?: Weight;
  readonly text?: undefined;
  readonly status?: undefined;
  readonly dot?: undefined;
}

export type AnyBadgeProps = BadgeProps | BadgeTierProps | BadgeCountProps;

export const CAP = 99;

export type BadgeLook =
  | {
      readonly form: "word";
      readonly text: string;
      readonly mark: GlyphName | "dot" | undefined;
      readonly tier: Weight;
    }
  | {
      readonly form: "count";

      readonly text: string | undefined;
      readonly tier: Weight;
      readonly quiet: Quiet;
    };

export interface Quiet {
  readonly "aria-hidden": "true";
}

export function badgeOf(props: AnyBadgeProps): BadgeLook | undefined {
  if (props.count !== undefined)
    return countOf(props.count, props.cap ?? CAP, props.weight ?? "alert");
  if (props.status !== undefined) {
    const look = statusLook(props.status);
    return { form: "word", text: props.text, mark: look.glyph, tier: look.weight };
  }
  return {
    form: "word",
    text: props.text,
    mark: props.dot === true ? "dot" : undefined,
    tier: props.weight ?? "quiet",
  };
}

function countOf(count: number | "fresh", cap: number, tier: Weight): BadgeLook | undefined {
  const quiet: Quiet = { "aria-hidden": "true" };
  if (count === "fresh") return { form: "count", text: undefined, tier, quiet };
  if (count <= 0) return undefined;
  return { form: "count", text: count > cap ? `${String(cap)}+` : String(count), tier, quiet };
}
