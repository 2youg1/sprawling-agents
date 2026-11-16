// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Address, AgentOffer, CloseMode, Command, DeviceId } from "../wire";
import { mintIdem } from "./idem";

export function forgetDevice(device: DeviceId): Command {
  return { forget_device: { device, idem: mintIdem() } };
}

export function addAgent(
  offer: Pick<AgentOffer, "spec_digest" | "source">,
  seatHere: Address | null,
): Command {
  return {
    add_agent: {
      spec_digest: offer.spec_digest,
      source: offer.source,
      seat_here: seatHere,
      idem: mintIdem(),
    },
  };
}

export function closeCity(mode: CloseMode): Command {
  return { close_city: { mode, idem: mintIdem() } };
}
