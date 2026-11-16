// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { DoctorItem } from "../../wire";

export interface DoctorLinkLook {
  readonly name: string;
  readonly homepage: string | null;
  readonly target: "_blank";
}

export function lookOf(item: DoctorItem): DoctorLinkLook {
  return {
    name: item.name,
    homepage: item.homepage ?? null,
    target: "_blank",
  };
}
