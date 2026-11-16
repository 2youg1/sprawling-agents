// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Attachment } from "svelte/attachments";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { LinkState } from "../../core/link";
import type { ButtonWire, Corner, EdgeKeyLook, Foot, Hint } from "../edge";
import { NO_CORNER, NO_FOOT } from "../edge";
import { linkWord } from "./link_word";

export interface Waiting {
  readonly needs: number;
  readonly fresh: boolean;
  readonly link: LinkState;
}

export type MailboxWire = Pick<ButtonWire, "aria-expanded" | "aria-controls" | "onclick"> &
  Readonly<Record<symbol, Attachment<HTMLElement>>>;

export function mailboxKey(
  waiting: Waiting,
  lang: Lang,
  hint: Hint,
  wire: MailboxWire,
): EdgeKeyLook {
  const name = nameOf(waiting, lang);
  return {
    glyph: "inbox",
    hint: hint(name),
    foot: footOf(waiting.link),
    corner: cornerOf(waiting),
    key: { as: "button", wire: { ...wire, type: "button", "aria-label": name } },
  };
}

function nameOf(waiting: Waiting, lang: Lang): string {
  const parts = [say(lang, "edge_mailbox")];
  if (waiting.needs > 0) parts.push(fill(say(lang, "nav_waiting"), { n: String(waiting.needs) }));
  else if (waiting.fresh) parts.push(say(lang, "mailbox_new"));
  if (waiting.link.kind !== "live") parts.push(linkWord(lang, waiting.link));
  return parts.join(" · ");
}

function cornerOf(waiting: Waiting): Corner {
  if (waiting.needs > 0) return { kind: "count", text: String(waiting.needs) };
  return waiting.fresh ? { kind: "fresh" } : NO_CORNER;
}

function footOf(link: LinkState): Foot {
  switch (link.kind) {
    case "live":
      return NO_FOOT;
    case "refused":
    case "closed":
      return { kind: "link", link: "refused" };
    case "idle":
    case "opening":
    case "handshaking":
    case "backoff":
      return { kind: "link", link: "connecting" };
  }
}
