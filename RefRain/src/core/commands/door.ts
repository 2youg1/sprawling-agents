// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { mintIdem } from "../idem";
import type { Command, IdemKey } from "../../wire";

export function openRemoteDoor(lastingMs: number): Command {
  return { open_remote_door: { lasting_ms: lastingMs, idem: mintIdem() } };
}

export function replaceCityKey(): Command {
  return { replace_city_key: { idem: mintIdem() } };
}

export function confirmRemoteDoor(code: string, approval: IdemKey): Command {
  return { confirm_remote_door: { code, approval, idem: mintIdem() } };
}

export function closeRemoteDoor(): Command {
  return { close_remote_door: { idem: mintIdem() } };
}
