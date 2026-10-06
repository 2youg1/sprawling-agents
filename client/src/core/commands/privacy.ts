// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The host privacy operation (`crates/wire/spec/Privacy.lean` §8-87). The
// host keeps each operation's result under its idem and answers it in
// `Query::Privacy`, so the idem travels back to the caller beside the
// command: the privacy page reads only the results of the idems it minted.

import { mintIdem } from "../idem";
import type { Command, IdemKey, PrivacyAction } from "../../wire";

export interface PrivacyOperation {
  readonly command: Command;
  readonly idem: IdemKey;
}

export function privacyOperation(action: PrivacyAction): PrivacyOperation {
  const idem = mintIdem();
  return { command: { privacy_operation: { action, idem } }, idem };
}
