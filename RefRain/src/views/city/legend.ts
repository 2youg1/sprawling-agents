// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The legend under the skyline: a city drawn in glyphs that nothing
// names is a picture, and the legend is the set of labels that makes it
// readable. It is folded behind one control, and its five marks are the
// drawing's own art rather than icons (client/Spec.lean §4-12).

import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";

// The five marks the drawing carries, in the order a reader meets them.
const MARKS = ["window", "figure", "flag", "lamp", "plinth"] as const;

export type LegendMark = (typeof MARKS)[number];

export interface LegendLook {
  readonly summary: string;
  readonly marks: readonly { readonly mark: LegendMark; readonly label: string }[];
}

export function legendOf(lang: Lang): LegendLook {
  return {
    summary: say(lang, "city_legend"),
    marks: MARKS.map((mark) => ({ mark, label: say(lang, `legend_${mark}`) })),
  };
}
