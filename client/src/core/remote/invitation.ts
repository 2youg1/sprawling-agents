// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface Invitation {
  readonly code: string;
  readonly city: Uint8Array;
}
export function isInvitation(_hash: string): boolean {
  return false;
}
export function invitationIn(_hash: string): Invitation | null {
  return null;
}
