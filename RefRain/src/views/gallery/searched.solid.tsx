// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import SearchCard from "../setup/search/search_view";
import { GalleryCase as Case } from "./case";
import { SEARCH, SEARCH_CUSTOM } from "./configured";

export default function Searched() {
  return (
    <>
      <Case label="search · the default service, nothing written">
        <SearchCard settled={SEARCH} />
      </Case>

      <Case label="search · two services of the city's own, the hall stating its own">
        <SearchCard settled={SEARCH_CUSTOM} />
      </Case>
    </>
  );
}
