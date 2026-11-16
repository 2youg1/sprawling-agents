// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Option } from "effect";

import { PAGES, page } from "./route";
import type { View } from "./route";
import type { Slash } from "./slash_hands";

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
      hands.go({ kind: "welcome" });
      hands.write("");
    },
  },
  {
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
