// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get } from "svelte/store";

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

export type Rgb = readonly [number, number, number];

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
  return { style: document.documentElement.style, sheet };
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

export function shortfalls(judged: Claims, resolve: (token: string) => Rgb | null): Shortfall[] {
  const surface = judged.surface === null ? null : resolve(judged.surface);
  if (surface === null) return [];
  return judged.claims.flatMap((claim): Shortfall[] => {
    const text = resolve(claim.token);
    if (text === null) return [];
    const reached = apcaLc(text, surface);
    return reached + judged.slack < claim.tier
      ? [{ token: claim.token, claimed: claim.tier, reached }]
      : [];
  });
}

export function apcaLc(text: Rgb, surface: Rgb): number {
  const textY = softClamp(luminance(text));
  const surfaceY = softClamp(luminance(surface));
  if (Math.abs(surfaceY - textY) < 0.0005) return 0;
  if (surfaceY > textY) {
    const sapc = (surfaceY ** 0.56 - textY ** 0.57) * 1.14;
    return sapc < 0.001 ? 0 : (sapc - 0.027) * 100;
  }
  const sapc = (surfaceY ** 0.65 - textY ** 0.62) * 1.14;
  return sapc > -0.001 ? 0 : Math.abs(sapc + 0.027) * 100;
}

function luminance([red, green, blue]: Rgb): number {
  return (
    0.2126729 * (red / 255) ** 2.4 +
    0.7151522 * (green / 255) ** 2.4 +
    0.072175 * (blue / 255) ** 2.4
  );
}

function softClamp(y: number): number {
  return y > 0.022 ? y : y + (0.022 - y) ** 1.414;
}

export function drawnRgb(root: HTMLElement): (token: string) => Rgb | null {
  const pen = root.ownerDocument
    .createElement("canvas")
    .getContext("2d", { willReadFrequently: true });
  const computed = window.getComputedStyle(root);
  return (token) => {
    const value = computed.getPropertyValue(token).trim();
    if (pen === null || value === "" || !CSS.supports("color", value)) return null;
    pen.clearRect(0, 0, 1, 1);
    pen.fillStyle = value;
    pen.fillRect(0, 0, 1, 1);
    const [red, green, blue] = pen.getImageData(0, 0, 1, 1).data;
    return red === undefined || green === undefined || blue === undefined
      ? null
      : [red, green, blue];
  };
}
