// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Both answers the settings page can give to "where does this choice
// live".
//
// The second one is the reason this file exists. A running client
// cannot reach it yet - `Query::Preferences` does not exist, so
// `PreferenceDoor.adopt` has no caller on the production path and
// `keeper()` always says `browser`. A fixture for a state only the
// fixture can reach is the whole point of this route: the badge is
// drawn, measured and read out at every page width the gate opens,
// so the day the city answers, the state it lands in has already been
// looked at.

import { Kept } from "../setup/kept";
import { Case } from "./case";

export function Keepers() {
  return (
    <>
      <Case label="kept · this browser, until the city answers">
        <Kept keeper="browser" />
      </Case>

      <Case label="kept · the person's own config.toml">
        <Kept keeper="city" />
      </Case>
    </>
  );
}
