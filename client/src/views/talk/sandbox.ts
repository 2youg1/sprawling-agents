// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The sandbox a room's building boxes a run in, read as one fact
// (docs/frontend-method.md §7D, §7I). The fact exists only where the
// sandbox restricts a run: a building with no sandbox, or one whose arm
// runs commands on the machine itself, has nothing to report, and a fact
// that says "none" is a word a person reads for nothing.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { buildingOf } from "../../core/route";
import type { Address, Answer, Query, SandboxLimits } from "../../wire";

// The question whose answer carries the sandbox of the room's building.
export function sandboxQuery(room: Address): Query {
  return { building_view: { addr: buildingOf(room) } };
}

// The limits that restrict a run in that building, or `null` where
// nothing does - including while the answer has not arrived, so the fact
// appears once it is known rather than as a placeholder.
export function restriction(held: Answer | undefined): SandboxLimits | null {
  if (held === undefined || !("building" in held)) return null;
  const limits = held.building.sandbox ?? null;
  return limits === null || limits.arm === "none" ? null : limits;
}

// The fact in words, the same on the settings row and the first message head.
export function sandboxSaid(limits: SandboxLimits, lang: Lang): string {
  return fill(say(lang, "facts_sandbox_said"), { n: String(limits.mounts.length) });
}
