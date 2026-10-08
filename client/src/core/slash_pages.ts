// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The verbs that only open a page, as rows of the one slash table
// (`slash.ts` spreads them into `SLASH`, which stays the only list a
// menu reads).

import { Option } from "effect";

import { PAGES, page } from "./route";
import type { View } from "./route";
import type { Slash } from "./slash_hands";

// Only what `/go` advertises, resolved by the router: a page this
// build cannot name is not a page this verb opens.
function paged(name: string | undefined): View | null {
  if (name === undefined || !PAGES.includes(name)) {
    return null;
  }
  return Option.getOrNull(page(name));
}

export const PAGE_VERBS: readonly Slash[] = [
  {
    spelling: "/go",
    grammar: PAGES.join("|"),
    about: "slash_go",
    section: "navigation",
    run: (hands, call) => {
      const to = paged(call.words.at(0));
      if (to === null) return;
      hands.go(to);
      hands.write("");
    },
  },
  {
    spelling: "/mcp",
    grammar: "",
    about: "slash_mcp",
    section: "navigation",
    run: (hands) => {
      hands.go({ kind: "mcp" });
      hands.write("");
    },
  },
  {
    // The ACP agents page; a launch spec typed after the verb is pasted
    // into the page's box by hand, because the box is where the city's
    // reading of it is shown before anything is added.
    spelling: "/acp",
    grammar: "",
    about: "slash_acp",
    section: "navigation",
    run: (hands) => {
      hands.go({ kind: "setup", group: "agents" });
      hands.write("");
    },
  },
  {
    spelling: "/doctor",
    grammar: "",
    about: "slash_doctor",
    section: "navigation",
    run: (hands) => {
      // The machine report is the welcome's first step, and that page
      // asks the `doctor` query again when it opens.
      hands.go({ kind: "welcome" });
      hands.write("");
    },
  },
  {
    // The changes are a lens of the run page: open this run's, or the newest here.
    spelling: "/diff",
    grammar: "",
    about: "slash_diff",
    section: "navigation",
    run: (hands) => {
      const shown = hands.live ?? (hands.here === null ? null : hands.newest(hands.here));
      if (shown === null) return;
      hands.go({ kind: "run", run: shown.run, lens: "changes" });
      hands.write("");
    },
  },
];
