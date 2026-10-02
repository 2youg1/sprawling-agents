// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The link's state in one word, read by the mailbox key's name and by
// the head of its column. Four of the six states are one word to a
// person: the socket is on its way up.

import type { LinkState } from "../../core/link";
import type { Lang } from "../../core/lang";
import { say } from "../../core/lang";

export function linkWord(lang: Lang, link: LinkState): string {
  switch (link.kind) {
    case "live":
      return say(lang, "link_live");
    case "refused":
      return say(lang, "link_refused");
    case "idle":
    case "opening":
    case "handshaking":
    case "backoff":
      return say(lang, "link_connecting");
  }
}
