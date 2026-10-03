// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The colour page's logic: the tokens it offers, how the person's
// override is worn and taken off, and the legibility it warns about
// (roadmap CT, `crates/wire/spec/Preference.lean` D29).
//
// **The theme is read off the page, never copied.** `theme.css` is
// compiled into the bundle, so the running page has no file to open;
// its `@theme` block is a set of CSS variables, and the stylesheets the
// document holds are where this file reads their names. The tiers a
// text token claims and the brightest surface text may sit on are read
// the same way, from the `--tier-*` and `--surface-ceiling` declarations
// `cargo xtask color` judges, so the gate and this page answer from one
// table and the page only warns where the gate would refuse.
//
// **An override is worn in two places.** Each token goes on the root's
// inline style, which outranks both lightings of the theme without a
// selector of its own, and the person's stylesheet goes into one style
// element after the built-in theme. Taking it off removes exactly those,
// so "restore default" is the built-in theme value by value.

import { get } from "svelte/store";

import type { PreferenceDoor } from "../../core/prefs";
import type { Theme } from "../../core/theme_override";

// The prefix the `@theme` block gives every colour token.
const TOKEN_PREFIX = "--color-";

// The declarations `theme.css` states for `xtask color` beside the
// values it judges: a text token's tier is `--tier-<name>` for
// `--color-<name>`, the ceiling names a rung of the grey ramp, and the
// slack is how far under its tier a reading may fall - the rounding the
// gate allows, so a value it passes is not warned about here.
const TIER_PREFIX = "--tier-";
const CEILING = "--surface-ceiling";
const SLACK = "--tier-slack";
const TEXT_TOKEN = `${TOKEN_PREFIX}text`;

// The one element the person's stylesheet is written into.
const SHEET_ID = "sprawling-theme-override";

// The parts of the root's inline style this file touches; a
// `CSSStyleDeclaration` is one.
export interface Inline {
  readonly length: number;
  readonly item: (index: number) => string;
  readonly setProperty: (name: string, value: string) => void;
  readonly removeProperty: (name: string) => string;
}

// Where an override is worn: the root's inline style and the one
// style element laid after the built-in theme.
export interface Stage {
  readonly style: Inline;
  readonly sheet: { textContent: string | null };
}

// A displayed colour, as eight-bit sRGB channels.
export type Rgb = readonly [number, number, number];

export interface Claim {
  readonly token: string;
  readonly tier: number;
}

// The text tokens and the tier each claims, and the surface they are
// judged on, or `null` when the theme names none.
export interface Claims {
  readonly surface: string | null;
  // Read off `--tier-slack`; a theme that states none allows none.
  readonly slack: number;
  readonly claims: readonly Claim[];
}

export interface Shortfall {
  readonly token: string;
  readonly claimed: number;
  readonly reached: number;
}

// ------------------------------------------------------------ the tokens

// The colour tokens among the declared variable names, once each, in
// the order the stylesheet first declares them: the ramp from dark to
// light, then the coloured tokens, then the text.
export function colourTokens(declared: Iterable<string>): string[] {
  return [...new Set([...declared].filter((name) => name.startsWith(TOKEN_PREFIX)))];
}

// The custom property names every rule of the document's stylesheets
// declares, `@layer` and `@media` blocks included.
export function declaredNames(document: Document): string[] {
  return [...document.styleSheets].flatMap((sheet) => namesIn(sheet.cssRules));
}

function namesIn(rules: CSSRuleList): string[] {
  return [...rules].flatMap((rule) => {
    const own = rule instanceof CSSStyleRule ? [...rule.style].filter((name) => name.startsWith("--")) : [];
    const nested = rule instanceof CSSGroupingRule ? namesIn(rule.cssRules) : [];
    return [...own, ...nested];
  });
}

// ------------------------------------------------------------ wearing

// Lays `theme` over the built-in one, taking off whatever token an
// earlier override laid and this one does not.
export function wearTheme(stage: Stage, theme: Theme): void {
  const worn = Array.from({ length: stage.style.length }, (_, at) => stage.style.item(at));
  for (const name of worn) {
    if (name.startsWith(TOKEN_PREFIX) && !(name in theme.tokens)) stage.style.removeProperty(name);
  }
  for (const [name, value] of Object.entries(theme.tokens)) stage.style.setProperty(name, value);
  stage.sheet.textContent = theme.css ?? "";
}

// Wears the kept override now and again whenever it changes, whether
// the person changed it here or the city's answer brought it. Called
// once, where the client starts, before the first paint.
export function wearKeptTheme(stage: Stage, door: PreferenceDoor): void {
  let worn = get(door.held).theme;
  wearTheme(stage, worn);
  door.held.subscribe((held) => {
    if (held.theme === worn) return;
    worn = held.theme;
    wearTheme(stage, worn);
  });
}

// The document's stage: the root's inline style and the one style
// element, made at the end of the head the first time it is asked for.
export function documentStage(document: Document): Stage {
  const found = document.getElementById(SHEET_ID);
  const sheet = found ?? document.head.appendChild(document.createElement("style"));
  sheet.id = SHEET_ID;
  return { style: document.documentElement.style, sheet };
}

// ---------------------------------------------------------- legibility

// The text tokens offered, each with the tier its `--tier-` twin
// claims, and the surface the ceiling names. A token without a stated
// tier claims nothing and is not judged.
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

// The text tokens that, as drawn now, do not reach their tier on the
// ceiling surface. A colour the page cannot resolve is not judged.
export function shortfalls(judged: Claims, resolve: (token: string) => Rgb | null): Shortfall[] {
  const surface = judged.surface === null ? null : resolve(judged.surface);
  if (surface === null) return [];
  return judged.claims.flatMap((claim): Shortfall[] => {
    const text = resolve(claim.token);
    if (text === null) return [];
    const reached = apcaLc(text, surface);
    return reached + judged.slack < claim.tier ? [{ token: claim.token, claimed: claim.tier, reached }] : [];
  });
}

// APCA lightness contrast (APCA-W3 0.0.98G-4g), as an absolute Lc: the
// measure `xtask color` judges the theme with, applied to whatever the
// person laid over it.
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
  return 0.2126729 * (red / 255) ** 2.4 + 0.7151522 * (green / 255) ** 2.4 + 0.072175 * (blue / 255) ** 2.4;
}

// APCA lifts the darkest values so near-black pairs do not report more
// contrast than an eye finds there.
function softClamp(y: number): number {
  return y > 0.022 ? y : y + (0.022 - y) ** 1.414;
}

// The colour a token draws now, as displayed sRGB: painted once onto a
// one-pixel canvas, which resolves every colour syntax the engine knows.
export function drawnRgb(root: HTMLElement): (token: string) => Rgb | null {
  const pen = root.ownerDocument.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const computed = window.getComputedStyle(root);
  return (token) => {
    const value = computed.getPropertyValue(token).trim();
    if (pen === null || value === "" || !CSS.supports("color", value)) return null;
    pen.clearRect(0, 0, 1, 1);
    pen.fillStyle = value;
    pen.fillRect(0, 0, 1, 1);
    const [red, green, blue] = pen.getImageData(0, 0, 1, 1).data;
    return red === undefined || green === undefined || blue === undefined ? null : [red, green, blue];
  };
}
