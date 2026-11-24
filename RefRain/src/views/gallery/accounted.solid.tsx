// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import Accounts from "../setup/providers/accounts.solid";
import { endpointRoster } from "../setup/providers/rosters";
import { GalleryCase } from "./case";
import { ACCOUNTED, UNLISTED } from "./configured";

export default function AccountsCases() {
  return (
    <>
      <GalleryCase label={"provider · accounts in order, each with its key"}>
        <Accounts roster={endpointRoster(ACCOUNTED, "two")} />
      </GalleryCase>
      <GalleryCase label={"provider · attached with one key, no accounts yet"}>
        <Accounts roster={endpointRoster(UNLISTED, "two")} />
      </GalleryCase>
    </>
  );
}
