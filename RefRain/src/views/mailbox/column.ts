// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


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
  
  
  readonly back: Press;
  readonly link: LinkLook | undefined;
  readonly scroller: ScrollerWire;
  
  readonly body: Snippet;
}

export interface Press {
  readonly label: string;
  readonly onPress: () => void;
}


export interface LinkLook {
  readonly word: string;
  readonly recovery: Press | undefined;
}


export type ScrollerWire = Readonly<Record<symbol, Attachment<HTMLElement>>>;


export function linkOf(
  link: LinkState,
  lang: Lang,
  recover: (lever: Recovery, error: AxError) => void,
): LinkLook | undefined {
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
    case "closed":
      return { word: linkWord(lang, link), recovery: undefined };
  }
}
