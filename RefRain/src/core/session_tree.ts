// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One session tree for the sessions pane, the zen switcher and the
// palette's session group. Parentage comes only from the sessions answer,
// never from the runs this page happens to hold (client D107).

import type { Stretch } from "./stretches";

export interface SessionNode {
  readonly stretch: Stretch;
  readonly key: string;
  readonly depth: number;
  readonly children: boolean;
  readonly delegated: boolean;
}

export function sessionKey(stretch: Stretch): string {
  return `${stretch.room}:${String(stretch.line.began)}`;
}

export function defaultCollapsed(stretches: readonly Stretch[]): ReadonlySet<string> {
  const known = new Set(stretches.map(sessionKey));
  return new Set(
    stretches.flatMap((stretch) => {
      const parent = stretch.line.parent;
      if (parent === undefined || parent === null || stretch.line.branched_from != null) return [];
      const key = `${parent.room}:${String(parent.began)}`;
      return known.has(key) ? [key] : [];
    }),
  );
}

export function collapsedOf(
  stretches: readonly Stretch[],
  overrides: Readonly<Record<string, boolean>>,
): ReadonlySet<string> {
  const held = new Set(
    [...defaultCollapsed(stretches)].filter((key) => overrides[key] === undefined),
  );
  for (const [key, closed] of Object.entries(overrides)) {
    if (closed) held.add(key);
  }
  return held;
}

interface Frame {
  readonly key: string;
  readonly depth: number;
  readonly exiting: boolean;
}

export function sessionTree(
  stretches: readonly Stretch[],
  collapsed: ReadonlySet<string>,
): SessionNode[] {
  const byKey = new Map(stretches.map((stretch) => [sessionKey(stretch), stretch]));
  const children = new Map<string | null, string[]>();
  for (const stretch of stretches) {
    const parent = stretch.line.parent;
    const raw =
      parent === undefined || parent === null ? null : `${parent.room}:${String(parent.began)}`;
    const key = raw !== null && byKey.has(raw) ? raw : null;
    children.set(key, [...(children.get(key) ?? []), sessionKey(stretch)]);
  }

  const roots = children.get(null) ?? [];
  const anchored = new Set<string>();
  const pending = [...roots];
  while (pending.length > 0) {
    const key = pending.pop();
    if (key === undefined || anchored.has(key)) continue;
    anchored.add(key);
    pending.push(...(children.get(key) ?? []));
  }
  const unrooted = stretches.flatMap((stretch) =>
    anchored.has(sessionKey(stretch)) ? [] : [sessionKey(stretch)],
  );
  const stack: Frame[] = [...roots, ...unrooted]
    .reverse()
    .map((key) => ({ key, depth: 0, exiting: false }));
  const path = new Set<string>();
  const visited = new Set<string>();
  const nodes: SessionNode[] = [];
  while (stack.length > 0) {
    const frame = stack.pop();
    if (frame === undefined) break;
    if (frame.exiting) {
      path.delete(frame.key);
      continue;
    }
    if (path.has(frame.key) || visited.has(frame.key)) continue;
    const stretch = byKey.get(frame.key);
    if (stretch === undefined) continue;
    visited.add(frame.key);
    const branches = children.get(frame.key) ?? [];
    nodes.push({
      stretch,
      key: frame.key,
      depth: frame.depth,
      children: branches.length > 0,
      delegated: stretch.line.parent != null && stretch.line.branched_from == null,
    });
    if (collapsed.has(frame.key)) continue;
    path.add(frame.key);
    stack.push({ key: frame.key, depth: frame.depth, exiting: true });
    for (const key of [...branches].reverse()) {
      stack.push({ key, depth: frame.depth + 1, exiting: false });
    }
  }
  return nodes;
}
