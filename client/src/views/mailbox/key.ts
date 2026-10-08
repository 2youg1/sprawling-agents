// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mailbox key's value for the edge key look (`../edge.ts`).
//
// **The key says three things and keeps them apart** (refrain 3-6): a
// number is what needs the person - questions, a door waiting on their
// hand, a failure that stops the work - and nothing else; an ordinary
// refusal nobody has read is a plain dot with no number, which is all
// `core/deferral.ts` lets it do until a moment comes; and the link, when
// it is not live, is a bar of its own whose name the key's name says,
// never a share of the count. The key's name says each reason it carries
// a mark, so the marks themselves are hidden from a screen reader.

import type { Attachment } from "svelte/attachments";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { LinkState } from "../../core/link";
import type { ButtonWire, Corner, EdgeKeyLook, Foot, Hint } from "../edge";
import { NO_CORNER, NO_FOOT } from "../edge";
import { linkWord } from "./link_word";

// What the key reads from the city.
export interface Waiting {
  // How many things stop until the person acts.
  readonly needs: number;
  // Whether an ordinary refusal nobody has read is there.
  readonly fresh: boolean;
  readonly link: LinkState;
}

// What the seat wires the key with: whether the column is open, the
// column's id, the press, and the hold that lets the seat put the focus
// back on the key.
export type MailboxWire = Pick<ButtonWire, "aria-expanded" | "aria-controls" | "onclick"> & Readonly<Record<symbol, Attachment<HTMLElement>>>;

export function mailboxKey(waiting: Waiting, lang: Lang, hint: Hint, wire: MailboxWire): EdgeKeyLook {
  const name = nameOf(waiting, lang);
  return {
    glyph: "inbox",
    hint: hint(name),
    foot: footOf(waiting.link),
    corner: cornerOf(waiting),
    key: { as: "button", wire: { ...wire, type: "button", "aria-label": name } },
  };
}

// What the key says, which is its whole name: what it is, then each
// reason it carries a mark.
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

// The link's own mark: the bar `core/mark.ts` draws on the tab for a
// page that is not being told. Still on its way up, the state that
// resolves by itself, it pulses rather than asking for anything.
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
