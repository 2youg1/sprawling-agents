// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The frozen facts a session states beside its model, in the words the
// first message head and the panorama's model cell both write: the
// effort its requests froze, then the mode it was dispatched in
// (client/Spec.lean §4-44, §4-58). Each is read from the run's opening; a fact
// the opening does not carry is left out rather than taken from the
// room's current setting.

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Opening } from "../../wire";

export function frozenSaid(opening: Opening | null | undefined, lang: Lang): string {
  const effort = opening?.effort ?? null;
  const mode = opening?.policy?.mode ?? null;
  return [effort === null ? "" : say(lang, `effort_${effort}`), mode === null ? "" : say(lang, `mode_${mode}`)]
    .filter((part) => part !== "")
    .join(" · ");
}
