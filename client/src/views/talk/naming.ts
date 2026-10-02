// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the conversation calls the resident it speaks with (refrain
// §3-13, client/Spec.lean §4-59). The Mayor is drawn under the name the person
// gave it, and under the language's default name when there is none;
// every other resident under the last segment of its address, which is
// the name its building gave it.
//
// Which name is the caller's to say: a session that has started passes
// the name it froze (`Opening.names`), and a room before its first send
// passes the name a new session would freeze (`Query::Identity`). A
// display name never moves an address, so `hall/mayor` stays in the
// details where it was.

import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { MAYOR, roomOf } from "../../core/route";
import type { Address } from "../../wire";

export function called(address: Address, mayor: string | null | undefined, lang: Lang): string {
  if (address !== MAYOR) return roomOf(address);
  return mayor ?? say(lang, "nav_mayor");
}
