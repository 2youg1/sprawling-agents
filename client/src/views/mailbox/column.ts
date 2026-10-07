// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the mailbox column's look is handed (client/Spec.lean §4-49): its
// head - the name, the link when it is not live with the one way to
// recover it the city named, and on a phone the way back - then the one
// scroller every section stands in, as a bag the look spreads on it.

import type { Snippet } from "svelte";
import type { Attachment } from "svelte/attachments";

import type { Lang } from "../../core/lang";
import type { LinkState } from "../../core/link";
import type { Recovery } from "../../core/recovering";
import { linkRecovery } from "../../core/recovering";
import { recoveryLabel } from "../notice_recovery";
import type { AxError } from "../../wire";
import { linkWord } from "./link_word";

export interface ColumnLook {
  readonly title: string;
  // The way back, drawn only on a phone, where the column is a whole
  // screen sheet (refrain U9).
  readonly back: Press;
  readonly link: LinkLook | undefined;
  readonly scroller: ScrollerWire;
  // The sections, in the order of what needs the person.
  readonly body: Snippet;
}

export interface Press {
  readonly label: string;
  readonly onPress: () => void;
}

// The link in one word, and the way to recover it when the city refused
// the page; a link on its way up resolves by itself and offers nothing.
export interface LinkLook {
  readonly word: string;
  readonly recovery: Press | undefined;
}

// Spread on the scroller: the seat's hold on it, which hears j, k and the
// digits and tells the recent sessions which rows are in view.
export type ScrollerWire = Readonly<Record<symbol, Attachment<HTMLElement>>>;

// `recover` is handed the lever and the refusal it answers when the
// person presses it.
export function linkOf(link: LinkState, lang: Lang, recover: (lever: Recovery, error: AxError) => void): LinkLook | undefined {
  switch (link.kind) {
    case "live":
      return undefined;
    case "refused": {
      const lever = linkRecovery(link.error.code);
      return {
        word: linkWord(lang, link),
        recovery: {
          label: recoveryLabel(lever, lang),
          onPress: () => {
            recover(lever, link.error);
          },
        },
      };
    }
    case "idle":
    case "opening":
    case "handshaking":
    case "backoff":
      return { word: linkWord(lang, link), recovery: undefined };
  }
}
