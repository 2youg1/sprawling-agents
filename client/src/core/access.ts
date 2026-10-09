// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The commands about who and what reaches the city, each a LocalOnly
// verb (`crates/wire/spec/Command/Kind.lean` §19-2): closing it,
// forgetting a paired browser and adding an ACP agent. Each carries its
// own idempotency key, as every command does. Starting an agent's
// sign-in has no sender until the city can run one
// (`crates/agent_protocols/Spec.lean` §3).

import type { Address, AgentOffer, CloseMode, Command, DeviceId } from "../wire";
import { mintIdem } from "./idem";

// Forget a paired browser: its key is dropped, and it pairs again to
// come back.
export function forgetDevice(device: DeviceId): Command {
  return { forget_device: { device, idem: mintIdem() } };
}

// Consent to one agent entry the city offered, named by the digest of
// the launch spec the card showed; `seatHere` also seats it in that room.
export function addAgent(offer: Pick<AgentOffer, "spec_digest" | "source">, seatHere: Address | null): Command {
  return { add_agent: { spec_digest: offer.spec_digest, source: offer.source, seat_here: seatHere, idem: mintIdem() } };
}

// Close the whole city: wait for the runs under way, or stop them now.
export function closeCity(mode: CloseMode): Command {
  return { close_city: { mode, idem: mintIdem() } };
}
