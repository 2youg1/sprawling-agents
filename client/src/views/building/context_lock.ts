// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the catalog's context pane reads and writes `context_lock_tokens`
// in a building's RULES.toml without becoming a second TOML parser: one
// line match for the key, and a whole-document rewrite that PutRules
// evaluates. The city remains the only reader of the schema.
//
// When the key is absent, the rewrite inserts it in the root table —
// before the first `[table]` header — never after a nested table such as
// `[workspace_models.choice]`, where `deny_unknown_fields` would refuse
// the save.

const KEY = "context_lock_tokens";
// TOML allows underscores in integers (`262_144`); strip them before Number.
const LINE = new RegExp(`^[ \\t]*${KEY}[ \\t]*=[ \\t]*(\\d(?:_?\\d)*)[ \\t]*(?:#.*)?$`, "m");
const ANY_LINE = new RegExp(`^[ \\t]*${KEY}[ \\t]*=[^\\n]*\\n?`, "gm");
const TABLE = /^[ \t]*\[[^\]\n]+\]/m;

/** The lock tokens stated in `text`, or `null` when the key is absent. */
export function lockOf(text: string): number | null {
  const match = LINE.exec(text);
  if (match === null) return null;
  const raw = match[1];
  if (raw === undefined) return null;
  const tokens = Number(raw.replaceAll("_", ""));
  return Number.isInteger(tokens) ? tokens : null;
}

/** `text` with `context_lock_tokens` set to `tokens`, or removed when `null`. */
export function withLock(text: string, tokens: number | null): string {
  const body = text.replace(/\r\n/g, "\n");
  const stripped = body.replace(ANY_LINE, "").replace(/\n{3,}/g, "\n\n");
  if (tokens === null) {
    return stripped.endsWith("\n") || stripped.length === 0 ? stripped : `${stripped}\n`;
  }
  const line = `${KEY} = ${String(tokens)}\n`;
  const table = TABLE.exec(stripped);
  if (table !== null) {
    const before = stripped.slice(0, table.index).replace(/\s*$/, "");
    const after = stripped.slice(table.index);
    const joined = before.length === 0 ? `${line}\n${after}` : `${before}\n\n${line}\n${after}`;
    return joined.replace(/\n{3,}/g, "\n\n");
  }
  const trimmed = stripped.replace(/\s*$/, "");
  return trimmed.length === 0 ? line : `${trimmed}\n\n${line}`;
}

/** Whole percent of `window` that `lock` is, floored; `null` when unknown. */
export function lockShare(lock: number, window: number): number | null {
  if (lock <= 0 || window <= 0) return null;
  return Math.min(100, Math.floor((lock / window) * 100));
}
