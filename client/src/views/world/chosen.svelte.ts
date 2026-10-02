// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which session the panorama workbench speaks for, and which commit in
// it a person picked (client/Spec.lean §7K). The session pane, the commits
// pane and the timeline all read the one answer here, so the lane the
// commits pane marks and the session the middle pane draws cannot be two
// different runs.
//
// **A commit names its session.** Picking one picks the run that made it
// in the room that run worked in, and the timeline finds the checkpoint
// by the commit's oid; the conversation moves to that room the same way
// a session row moves it, through the address bar. A pick belongs to the
// room it was made in: once the conversation is somewhere else, the
// session there is that room's newest again.

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

// The commit picked in `room`, or null.
export function pickedCommit(room: Address): GitOid | null {
  return picked !== null && picked.room === room ? picked.oid : null;
}

// The run the workbench speaks for in `room`: the one a picked commit
// named, else the room's run still working, else the last it held.
export function sessionRun(belief: Belief, room: Address): RunId | null {
  if (picked !== null && picked.room === room) return picked.run;
  return (newestWorking(belief, room) ?? heldIn(belief, room).at(-1))?.run ?? null;
}

// The commits the workbench draws beside `room`: the whole city's from
// the Mayor's room, which is the city's own place, and the building's
// from any room inside one (refrain roadmap Q10).
export function commitsIn(room: Address): Query {
  return commitsQuery(room === MAYOR ? null : buildingOf(room), null);
}
