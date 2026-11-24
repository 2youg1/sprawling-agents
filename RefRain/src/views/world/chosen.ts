// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import { commitsQuery } from "../../core/asking";
import type { Belief } from "../../core/belief";
import { newestWorking } from "../../core/belief/live";
import { heldIn } from "../../core/belief/rooms";
import { MAYOR, buildingOf } from "../../core/route";
import type { Ui } from "../../ui";
import type { Address, CommitAnswer, GitOid, Query, RunId } from "../../wire";

interface Pick {
  readonly room: Address;
  readonly run: RunId;
  readonly oid: GitOid;
}

const picked = writable<Pick | null>(null);
export const commitSelection = readonly(picked);
const fixtureSelections = new WeakMap<Ui, typeof picked>();

export function commitStateOf(scope: Ui | undefined): typeof picked {
  if (scope === undefined) return picked;
  const existing = fixtureSelections.get(scope);
  if (existing !== undefined) return existing;
  const state = writable<Pick | null>(null);
  fixtureSelections.set(scope, state);
  return state;
}

export function pickCommit(commit: CommitAnswer, selection = picked): void {
  selection.set({ room: commit.actor, run: commit.run, oid: commit.oid });
}

export function pickedCommit(room: Address, commit = get(picked)): GitOid | null {
  return commit !== null && commit.room === room ? commit.oid : null;
}

export function sessionRun(belief: Belief, room: Address, commit = get(picked)): RunId | null {
  if (commit !== null && commit.room === room) return commit.run;
  return (newestWorking(belief, room) ?? heldIn(belief, room).at(-1))?.run ?? null;
}

export function commitsIn(room: Address): Query {
  return commitsQuery(room === MAYOR ? null : buildingOf(room), null);
}
