// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The person's preferences as the city keeps them, in the person's own
// `~/.sprawling/config.toml`: the one place this client's record and
// the wire's record meet.

import type { Preferences } from "./prefs";
import type { PreferencesAnswer } from "../wire";

// The city's answer taken over what this browser held. A field the
// answer leaves out is one the person never settled with the city, so
// the browser's value stands for it.
export function adopted(held: Preferences, _answer: PreferencesAnswer): Preferences {
  return held;
}
