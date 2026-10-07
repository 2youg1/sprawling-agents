// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The colours the person laid over the built-in theme, and the one
// reading of the row this browser caches them in.

import { Option, Schema } from "effect";

import { ThemeOverride } from "../wire";

// The colours the person laid over the built-in theme: CSS colours by
// `@theme` variable, and a stylesheet of their own laid after them
// (`crates/wire/spec/Preference.lean` D29). Whole rather than the wire's
// optional fields, so a reader never asks whether absent means empty.
export interface Theme {
  readonly tokens: Readonly<Record<string, string>>;
  readonly css: string | null;
}

// Nothing laid over the theme the client ships: what "restore default" writes.
export const BUILT_IN_THEME: Theme = { tokens: {}, css: null };

export function themeOf(stated: ThemeOverride): Theme {
  return { tokens: stated.tokens ?? {}, css: stated.css ?? null };
}

const readThemeRow = Schema.decodeOption(Schema.fromJsonString(ThemeOverride));

export function readTheme(raw: string | null): Theme {
  return raw === null ? BUILT_IN_THEME : Option.match(readThemeRow(raw), { onNone: () => BUILT_IN_THEME, onSome: themeOf });
}
