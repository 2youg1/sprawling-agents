// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import type { Notice } from "../../core/belief";

const swept = writable<Readonly<Record<string, number>>>({});
export const noticeSweeps = readonly(swept);

export function shown(notice: Notice, prior = get(swept)): boolean {
  return notice.count > (prior[notice.key] ?? 0);
}

export function sweep(notices: readonly Notice[]): void {
  const next: Record<string, number> = { ...get(swept) };
  for (const notice of notices) next[notice.key] = notice.count;
  swept.set(next);
}
