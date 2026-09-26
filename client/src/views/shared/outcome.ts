// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How an outcome is marked wherever a result is listed: the results city
// and the results room draw the same glyph in the same ink, so a person
// who learned one reads the other.

import type { Outcome } from "../../core/results";
import type { GlyphName } from "../parts/glyph";

export const OUTCOME_GLYPH: Record<Outcome, GlyphName> = {
  waiting: "hand",
  failed: "cross",
  done: "check",
  ended: "ring",
};

export const OUTCOME_INK: Record<Outcome, string> = {
  waiting: "text-alert",
  failed: "text-alert",
  done: "text-text-quiet",
  ended: "text-text-faint",
};
