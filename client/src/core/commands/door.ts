// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The remote door's four commands, apart from `commands.ts` for the
// length budget that file sits at. Opening and replacing the key only
// ask: the city prints a code on its own console and answers
// `E_APPROVAL_PENDING`, and the code comes back in `confirmRemoteDoor`
// (`crates/remote_access/Spec.lean` D4). Closing needs no code.

import { mintIdem } from "../idem";
import type { Command } from "../../wire";

export function openRemoteDoor(lastingMs: number): Command {
  return { open_remote_door: { lasting_ms: lastingMs, idem: mintIdem() } };
}

export function replaceCityKey(): Command {
  return { replace_city_key: { idem: mintIdem() } };
}

export function confirmRemoteDoor(code: string): Command {
  return { confirm_remote_door: { code, idem: mintIdem() } };
}

export function closeRemoteDoor(): Command {
  return { close_remote_door: { idem: mintIdem() } };
}
