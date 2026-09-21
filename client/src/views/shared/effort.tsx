// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How hard the city thinks, and the three things a person has to know
// before they choose: the choice is frozen for the length of a run,
// the two wire formats disagree about where `none` is written, and
// saying nothing is not the same as saying `none`.
//
// **There is no picker here, because this page cannot keep an
// answer.** The standing level is the city's own `[model] effort`,
// read from `CONFIG.toml` by the configuration ladder that freezes
// every run; a level kept in this browser instead would ride on every
// dispatch from it and outrank the file without saying so. What this
// build can state about the file it can neither read (`Query::Config`,
// roadmap 3.3) nor write, so the section says where the answer lives
// and stops there.
//
// **A session states its own level in the selector over the
// composer**, which is where a dispatch is made and the only scope
// this client can honestly offer: the city writes that level into the
// room it opens, and every run in that room then holds it.
//
// The welcome walk and the settings page both show this, so the
// paragraph cannot be present on one page and missing from the other.
// The words are in `lang.json` like every other word a reader is
// handed - this file only says where they go.

import { useSay } from "../../ui";

export function EffortSection() {
  const say = useSay();
  return (
    <div class="flex flex-col gap-base">
      <p class="text-note text-text-quiet">{say("setup_effort")}</p>
      <p class="max-w-measure text-note leading-relaxed text-text-faint">{say("setup_effort_essay")}</p>
      <p class="max-w-measure text-note text-text-quiet">{say("setup_effort_city")}</p>
    </div>
  );
}
