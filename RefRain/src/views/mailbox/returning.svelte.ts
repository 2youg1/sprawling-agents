// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

let wanted = $state<string | null>(null);

export function returnTo(row: string): void {
  wanted = row;
}

export function wantedRow(): string | null {
  return wanted;
}

export function takeRow(): void {
  wanted = null;
}
