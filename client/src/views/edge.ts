// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What an edge key's look is handed (docs/frontend-method.md §7E): the
// glyph, the hint that names the key and its chord, the marks it
// carries, and the key's wiring as one bag the look spreads on the
// element it draws. The three keys - layers, mailbox, settings - are one
// look with three values, so the column of keys reads as one control
// drawn three times. The layers and settings values are built here; the
// mailbox's is `mailbox/key.ts`'s, because what it counts is the
// mailbox's business.

import type { Attachment } from "svelte/attachments";
import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

import type { Lang, Key } from "../core/lang";
import { say } from "../core/lang";
import { TIERS } from "../core/prefs";
import type { Tier } from "../core/prefs";
import type { GlyphName } from "./parts/glyph";

export interface EdgeKeyLook {
  readonly glyph: GlyphName;
  // The key's name and its chord, already in the person's language;
  // drawn beside the key on hover, on focus and while the accelerator
  // is held alone.
  readonly hint: string;
  readonly foot: Foot;
  readonly corner: Corner;
  readonly key: KeyWire;
}

// What stands under the glyph: the three tier ticks of the layers key,
// the bar the mailbox key carries while the link is not live, or
// nothing.
export type Foot =
  | { readonly kind: "none" }
  | { readonly kind: "tiers"; readonly ticks: readonly Tick[] }
  | { readonly kind: "link"; readonly link: "connecting" | "refused" };

export interface Tick {
  readonly key: Tier;
  readonly held: boolean;
}

// What stands at the key's top right corner: how many things need the
// person, a dot for something new that has no number, or nothing.
export type Corner = { readonly kind: "none" } | { readonly kind: "count"; readonly text: string } | { readonly kind: "fresh" };

// A key is a button, or a link when it goes to a page, so a middle click
// and a new tab work as they do on every other link.
export type KeyWire = { readonly as: "button"; readonly wire: ButtonWire } | { readonly as: "link"; readonly wire: LinkWire };

export type ButtonWire = Pick<
  HTMLButtonAttributes,
  "type" | "aria-label" | "aria-expanded" | "aria-controls" | "onclick" | "onpointerdown" | "onpointerup" | "onpointerleave"
> &
  // The seat's hold on the drawn element, when it moves the focus there.
  Readonly<Record<symbol, Attachment<HTMLElement>>>;

export type LinkWire = Pick<HTMLAnchorAttributes, "href" | "aria-label">;

export const NO_FOOT: Foot = { kind: "none" };
export const NO_CORNER: Corner = { kind: "none" };

const TIER_NAME: Readonly<Record<Tier, Key>> = {
  zen: "tier_zen",
  blend: "tier_blend",
  panorama: "tier_panorama",
};

// The words a key's hint carries, given the key's name: the seat writes
// the chord that reaches the key beside it, from the one key table.
export type Hint = (name: string) => string;

// The layers key's three hands: a press that may become a hold, its
// release, and the click that changes the tier unless the press was a
// look.
export interface LayersHands {
  readonly press: (event: PointerEvent) => void;
  readonly release: () => void;
  readonly click: () => void;
}

// The layers key: named with the tier the page is drawn in now, and one
// tick per tier with the current one held.
export function layersKey(tier: Tier, lang: Lang, hint: Hint, hands: LayersHands): EdgeKeyLook {
  const name = `${say(lang, "edge_layers")} · ${say(lang, TIER_NAME[tier])}`;
  return {
    glyph: "layers",
    hint: hint(name),
    foot: { kind: "tiers", ticks: TIERS.map((each) => ({ key: each, held: each === tier })) },
    corner: NO_CORNER,
    key: {
      as: "button",
      wire: {
        type: "button",
        "aria-label": name,
        onpointerdown: hands.press,
        onpointerup: hands.release,
        onpointerleave: hands.release,
        onclick: hands.click,
      },
    },
  };
}

// The settings key: a link to the settings sheet.
export function settingsKey(lang: Lang, hint: Hint, href: string): EdgeKeyLook {
  const name = say(lang, "nav_settings");
  return {
    glyph: "settings",
    hint: hint(name),
    foot: NO_FOOT,
    corner: NO_CORNER,
    key: { as: "link", wire: { href, "aria-label": name } },
  };
}
