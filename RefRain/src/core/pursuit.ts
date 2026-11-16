// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The clause a person reads about a pursuit. The city decides the verdict
// and sends its kind; the words are this client's, one entry per kind in
// `lang.json`, so a Chinese page never has to show the city's English.

import type { PursuitVerdict } from "../wire";
import { fill, say, type Lang } from "./lang";

export function pursuitClause(lang: Lang, verdict: PursuitVerdict): string {
  switch (verdict.kind) {
    case "work":
      return fill(say(lang, "bld_pursuit_working"), { node: verdict.next });
    case "waiting":
      return fill(say(lang, "bld_pursuit_waiting"), { runs: String(verdict.in_flight) });
    case "paused":
      return say(lang, "bld_pursuit_paused");
    case "finished":
      return say(lang, "bld_pursuit_finished");
  }
}
