// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The frozen facts a session states beside its model, in the words the
// first message head and the panorama's model cell both write: the
// effort its requests froze, then its mode
// (client/Spec.lean §4-44, §4-58). Each is read from the run's opening; a fact
// the opening does not carry is left out rather than taken from the
// room's current setting.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Opening } from "../../wire";
import { policyFace } from "./policy";

function generationSaid(opening: Opening | null | undefined, lang: Lang): string[] {
  const frozen = opening?.generation;
  if (frozen == null) return [];
  const entries = [
    ["temperature", frozen.settings?.temperature],
    ["top_p", frozen.settings?.top_p],
    ["top_k", frozen.settings?.top_k],
    ["max_tokens", frozen.max_tokens?.toString()],
  ] as const;
  return entries
    .filter((entry) => entry[1] != null && entry[1] !== "")
    .map(([field, value]) =>
      fill(say(lang, "frozen_generation_value"), { field, value: value ?? "" }),
    );
}

export function frozenSaid(opening: Opening | null | undefined, lang: Lang): string {
  const effort = opening?.effort ?? null;
  const mode = opening?.policy?.mode ?? null;
  return [
    effort === null ? "" : say(lang, `effort_${effort}`),
    ...generationSaid(opening, lang),
    mode === null ? "" : say(lang, `mode_${mode}`),
  ]
    .filter((part) => part !== "")
    .join(" · ");
}

export function firstHeadSaid(opening: Opening | null | undefined, lang: Lang): string {
  return [
    frozenSaid(opening, lang),
    opening?.policy == null ? "" : policyFace(lang, opening.policy),
  ]
    .filter((part) => part !== "")
    .join(" · ");
}
