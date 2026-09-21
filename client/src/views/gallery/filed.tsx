// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this city decided is worth keeping, in the three readings the
// screen has: assets on it, nothing on it, and the same assets on a
// page a person asked to tighten.
//
// **The three rows arrive out of order on purpose.** The screen puts
// the newest first, and a fixture already sorted proves nothing about
// whether it does: the order below is the order a ledger hands them
// over, which is the order they were written in.
//
// The compact case draws the same three rows, so a reader comparing it
// with the case above it is comparing two drawings of one answer rather
// than two answers.

import type { RegistryLine } from "../../wire";
import { Address, TimeMs } from "../../wire";
import { RegistryTable } from "../registry";
import { Case } from "./case";

function filed(at: number, addr: string, kind: string, subject: string): RegistryLine {
  return { addr: Address.make(addr), at: TimeMs.make(at), kind, subject };
}

// Three assets, oldest in the middle, so the newest-first rule has
// something to do. The moments are fixed rather than taken from the
// clock: a fixture whose rows change every time the page opens is a
// fixture nobody can compare against yesterday's.
const ASSETS: readonly RegistryLine[] = [
  filed(1_764_500_400_000, "lab/east", "screenshot", "the settings page at 2560 px, forced colours"),
  filed(1_764_412_800_000, "hall/mayor", "transcript", "what the mayor was asked on the first day"),
  filed(1_764_586_800_000, "lab/west", "release", "sprawling 0.0.6, windows-x86_64"),
];

export function Filed() {
  return (
    <>
      <Case label="registry · three assets, handed over out of order">
        <RegistryTable assets={ASSETS} />
      </Case>

      <Case label="registry · nothing has been filed">
        <RegistryTable assets={[]} />
      </Case>

      {/* A compact page draws a row's title and drops the line that
          only elaborates it: here the two columns marked `summary` -
          when it was filed, and what kind of thing it is.
          `theme.css` owns that rule and states it once; this fixture
          only asks for the posture, so that the tightened reading has
          a reader that is not a person remembering to switch their own
          appearance over. The rule is written at `:root` today, so the
          fixture is the posture and the whole page is still where it
          takes effect. */}
      <Case label="registry · the same three rows, compact">
        <div data-density="compact">
          <RegistryTable assets={ASSETS} />
        </div>
      </Case>
    </>
  );
}
