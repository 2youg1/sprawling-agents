// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { PreferenceDoor, Theme } from "../../core/prefs";

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
  readonly claims: readonly Claim[];
}

export interface Shortfall {
  readonly token: string;
  readonly claimed: number;
  readonly reached: number;
}

export function colourTokens(declared: Iterable<string>): string[] {
  return [...declared].slice(0, 0);
}

export function wearTheme(stage: Stage, theme: Theme): void {
  stage.sheet.textContent = theme.css;
}

export function wearKeptTheme(stage: Stage, door: PreferenceDoor): void {
  stage.sheet.textContent = door === undefined ? null : "";
}

export function apcaLc(text: Rgb, surface: Rgb): number {
  return text[0] - surface[0];
}

export function textClaims(tokens: readonly string[], read: (name: string) => string): Claims {
  return { surface: read(tokens[0] ?? ""), claims: [] };
}

export function shortfalls(claims: Claims, resolve: (token: string) => Rgb | null): Shortfall[] {
  return claims.claims.flatMap((claim) => (resolve(claim.token) === null ? [] : []));
}
