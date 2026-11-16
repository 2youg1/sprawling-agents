// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";

export function nothingFound(query: string): Key {
  switch (query.trim().toLowerCase()) {
    case "harvey":
      return "palette_no_harvey";
    case "tafuri":
      return "palette_no_tafuri";
    default:
      return "part_no_match";
  }
}
