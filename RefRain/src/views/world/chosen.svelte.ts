// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { commitsQuery } from "../../core/asking";
import type { Belief } from "../../core/belief";
import { newestWorking } from "../../core/belief/live";
import { heldIn } from "../../core/belief/rooms";
import { MAYOR, buildingOf } from "../../core/route";
import type { Address, CommitAnswer, GitOid, Query, RunId } from "../../wire";

interface Pick {
  readonly room: Address;
  readonly run: RunId;
  readonly oid: GitOid;
}

let picked = $state<Pick | null>(null);

export function pickCommit(commit: CommitAnswer): void {
  picked = { room: commit.actor, run: commit.run, oid: commit.oid };
}

export function pickedCommit(room: Address): GitOid | null {
  return picked !== null && picked.room === room ? picked.oid : null;
}

export function sessionRun(belief: Belief, room: Address): RunId | null {
  if (picked !== null && picked.room === room) return picked.run;
  return (newestWorking(belief, room) ?? heldIn(belief, room).at(-1))?.run ?? null;
}

export function commitsIn(room: Address): Query {
  return commitsQuery(room === MAYOR ? null : buildingOf(room), null);
}
