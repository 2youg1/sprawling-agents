// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one row of a building's rooms is given (client D95: `rooms.svelte`
// is the seat, `room_row.look.svelte` draws the row): the room's name,
// how many runs work in it, and the press that opens its directory.

export interface RoomRowWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface RoomRowLook {
  readonly name: string;
  // The count of runs working at or below the room, as words, while
  // there are any.
  readonly live: string | undefined;
  readonly wire: RoomRowWire;
}
