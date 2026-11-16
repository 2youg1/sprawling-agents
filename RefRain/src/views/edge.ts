// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Attachment } from "svelte/attachments";
import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

import type { Lang, Key } from "../core/lang";
import { say } from "../core/lang";
import { TIERS } from "../core/prefs";
import type { Tier } from "../core/prefs";
import type { GlyphName } from "./parts/glyph";

export interface EdgeKeyLook {
  readonly glyph: GlyphName;
  readonly hint: string;
  readonly foot: Foot;
  readonly corner: Corner;
  readonly key: KeyWire;
}

export type Foot =
  | { readonly kind: "none" }
  | { readonly kind: "tiers"; readonly ticks: readonly Tick[] }
  | { readonly kind: "link"; readonly link: "connecting" | "refused" };

export interface Tick {
  readonly key: Tier;
  readonly held: boolean;
}

export type Corner =
  | { readonly kind: "none" }
  | { readonly kind: "count"; readonly text: string }
  | { readonly kind: "fresh" };

export type KeyWire =
  | { readonly as: "button"; readonly wire: ButtonWire }
  | { readonly as: "link"; readonly wire: LinkWire };

export type ButtonWire = Pick<
  HTMLButtonAttributes,
  | "type"
  | "aria-label"
  | "aria-expanded"
  | "aria-controls"
  | "onclick"
  | "onpointerdown"
  | "onpointerup"
  | "onpointerleave"
> &
  Readonly<Record<symbol, Attachment<HTMLElement>>>;

export type LinkWire = Pick<HTMLAnchorAttributes, "href" | "aria-label">;

export const NO_FOOT: Foot = { kind: "none" };
export const NO_CORNER: Corner = { kind: "none" };

const TIER_NAME: Readonly<Record<Tier, Key>> = {
  zen: "tier_zen",
  blend: "tier_blend",
  panorama: "tier_panorama",
};

export type Hint = (name: string) => string;

export interface LayersHands {
  readonly press: (event: PointerEvent) => void;
  readonly release: () => void;
  readonly click: () => void;
}

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
