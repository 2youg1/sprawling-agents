// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get } from "../../core/store";

import { apcaLc, over, type Rgb, type Rgba } from "../../core/apca";
import type { PreferenceDoor } from "../../core/prefs";
import type { Theme } from "../../core/theme_override";

const TOKEN_PREFIX = "--color-";

const TIER_PREFIX = "--tier-";
const CEILING = "--surface-ceiling";
const SLACK = "--tier-slack";
const TEXT_TOKEN = `${TOKEN_PREFIX}text`;

const SHEET_ID = "sprawling-theme-override";

export interface Inline {
  readonly length: number;
  readonly item: (index: number) => string;
  readonly setProperty: (name: string, value: string) => void;
  readonly removeProperty: (name: string) => string;
}

export interface Stage {
  readonly style: Inline;
  readonly sheet: { textContent: string | null };
}

export interface Claim {
  readonly token: string;
  readonly tier: number;
}

export interface Claims {
  readonly surface: string | null;

  readonly slack: number;
  readonly claims: readonly Claim[];
}

export interface Shortfall {
  readonly token: string;
  readonly claimed: number;
  readonly reached: number;
}

export function colourTokens(declared: Iterable<string>): string[] {
  return [...new Set([...declared].filter((name) => name.startsWith(TOKEN_PREFIX)))];
}

export function declaredNames(document: Document): string[] {
  return [...document.styleSheets].flatMap((sheet) => namesIn(sheet.cssRules));
}

function namesIn(rules: CSSRuleList): string[] {
  return [...rules].flatMap((rule) => {
    const own =
      rule instanceof CSSStyleRule ? [...rule.style].filter((name) => name.startsWith("--")) : [];
    const nested = rule instanceof CSSGroupingRule ? namesIn(rule.cssRules) : [];
    return [...own, ...nested];
  });
}

export function wearTheme(stage: Stage, theme: Theme): void {
  const worn = Array.from({ length: stage.style.length }, (_, at) => stage.style.item(at));
  for (const name of worn) {
    if (name.startsWith(TOKEN_PREFIX) && !(name in theme.tokens)) stage.style.removeProperty(name);
  }
  for (const [name, value] of Object.entries(theme.tokens)) stage.style.setProperty(name, value);
  stage.sheet.textContent = theme.css ?? "";
}

export function wearKeptTheme(stage: Stage, door: PreferenceDoor): void {
  let worn = get(door.held).theme;
  wearTheme(stage, worn);
  door.held.subscribe((held) => {
    if (held.theme === worn) return;
    worn = held.theme;
    wearTheme(stage, worn);
  });
}

export function documentStage(document: Document): Stage {
  const found = document.getElementById(SHEET_ID);
  const sheet = found ?? document.head.appendChild(document.createElement("style"));
  sheet.id = SHEET_ID;
  const inline = document.documentElement.style;
  return {
    sheet,
    style: {
      get length() {
        return inline.length;
      },
      item: (index) => inline.item(index),
      setProperty: (name, value) => {
        inline.setProperty(name, cssOklch(value));
      },
      removeProperty: (name) => inline.removeProperty(name),
    },
  };
}

export function textClaims(tokens: readonly string[], read: (name: string) => string): Claims {
  const rung = read(CEILING).trim();
  const surface = `${TOKEN_PREFIX}${rung}`;
  const claims = tokens.flatMap((token): Claim[] => {
    if (!token.startsWith(TEXT_TOKEN)) return [];
    const tier = Number(read(`${TIER_PREFIX}${token.slice(TOKEN_PREFIX.length)}`).trim());
    return Number.isFinite(tier) && tier > 0 ? [{ token, tier }] : [];
  });
  const slack = Number(read(SLACK).trim());
  return {
    surface: rung !== "" && tokens.includes(surface) ? surface : null,
    slack: Number.isFinite(slack) && slack > 0 ? slack : 0,
    claims,
  };
}

export type Reading =
  | { readonly kind: "unmeasured"; readonly token: string }
  | ({ readonly kind: "shortfall" } & Shortfall);

export function shortfalls(judged: Claims, resolve: (token: string) => Rgba | null): Reading[] {
  const surface = judged.surface === null ? null : resolve(judged.surface);
  const page = resolve("--color-page");
  const backdrop: Rgb | null = page?.[3] === 1 ? [page[0], page[1], page[2]] : null;
  const background =
    surface === null
      ? null
      : surface[3] === 1
        ? ([surface[0], surface[1], surface[2]] as const)
        : backdrop === null
          ? null
          : over(surface, backdrop);
  return judged.claims.flatMap((claim): Reading[] => {
    const text = resolve(claim.token);
    if (text === null || background === null) {
      return [{ kind: "unmeasured", token: claim.token }];
    }
    const reached = apcaLc(over(text, background), background);
    return Math.abs(reached) + judged.slack < claim.tier
      ? [{ kind: "shortfall", token: claim.token, claimed: claim.tier, reached }]
      : [];
  });
}

export function drawnRgb(root: HTMLElement): (token: string) => Rgba | null {
  const pen = root.ownerDocument
    .createElement("canvas")
    .getContext("2d", { colorSpace: "srgb", willReadFrequently: true });
  const computed = window.getComputedStyle(root);
  return (token) => {
    const value = computed.getPropertyValue(token).trim();
    if (pen === null || value === "" || !CSS.supports("color", value)) return null;
    pen.clearRect(0, 0, 1, 1);
    pen.fillStyle = "transparent";
    pen.fillStyle = cssOklch(value);
    pen.fillRect(0, 0, 1, 1);
    const [red, green, blue, alpha] = pen.getImageData(0, 0, 1, 1).data;
    return red === undefined || green === undefined || blue === undefined || alpha === undefined
      ? null
      : [red, green, blue, alpha / 255];
  };
}

function cssOklch(value: string): string {
  const relative = `oklch(from ${value} l c h / alpha)`;
  return typeof CSS !== "undefined" && CSS.supports("color", relative) ? relative : value;
}
