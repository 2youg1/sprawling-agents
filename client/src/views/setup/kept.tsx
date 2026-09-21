// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the answers a settings group collects are kept, said beside
// that group's name.
//
// A person who picks a face for the page, rebinds a chord or settles a
// proxy rule is making a choice they expect to find again. Until the
// city keeps these (roadmap 3.1) the only keeper is this browser, so
// clearing its data loses every one of them and another browser
// reaching the same city starts from the postures the client ships
// with. That was true before this badge and nothing on the page said
// it.
//
// The word comes from the door rather than from the group: the keeper
// is one fact about the whole record, and a group that reads the door
// reads whichever keeper the door reports.

import type { Key } from "../../core/lang";
import type { Keeper } from "../../core/prefs";
import { useSay } from "../../ui";
import { Badge } from "../parts/badge";

// One word per keeper, keyed by the keeper rather than listed in pairs:
// a keeper added to `core/prefs.ts` leaves this table refusing to
// compile until somebody gives it a word.
const WORDS: Record<Keeper, Key> = {
  browser: "setup_kept_browser",
  city: "setup_kept_city",
};

export function Kept(props: { readonly keeper: Keeper }) {
  const say = useSay();
  // Quiet in both states: where a preference lives is a fact about the
  // page, not a warning about it, and the city keeping them is the
  // ordinary case this client is heading for.
  return <Badge text={say(WORDS[props.keeper])} />;
}
