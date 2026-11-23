// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { buildingOf } from "../../core/route";
import type { Address, Answer, Query, SandboxLimits } from "../../wire";

export function sandboxQuery(room: Address): Query {
  return { building_view: { addr: buildingOf(room) } };
}

export function restriction(held: Answer | undefined): SandboxLimits | null {
  if (held === undefined || !("building" in held)) return null;
  const limits = held.building.sandbox ?? null;
  return limits === null || limits.arm === "none" ? null : limits;
}

export function sandboxSaid(_limits: SandboxLimits, lang: Lang): string {
  return say(lang, "facts_sandbox_said");
}
