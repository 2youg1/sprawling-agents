// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The first-run guide's one command, apart from `commands.ts` for the
// length budget that file sits at.

import { mintIdem } from "../idem";
import type { Command, GuideProgress } from "../../wire";

// This city's first-run guide progress, whole: where it reopens, whether
// the person left it, each optional step seen or skipped. The later of
// two writes stays (`crates/wire/Spec.lean` §8-68).
export function putGuide(progress: GuideProgress): Command {
  return { put_guide: { progress, idem: mintIdem() } };
}
