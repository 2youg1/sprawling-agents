// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every state worth looking at, on fixtures, with no city behind it.
//
// It is a route rather than a build flag for two reasons: the gate that
// measures it opens the same bundle a person runs, and a person deciding
// between two ways a thing could look can open it on their own machine
// without standing up a provider first.
//
// **This file is the index and nothing else.** Each section under
// `gallery/` owns one subject - the room a person works in, what a run
// produced, the registry, the shared controls - together with the
// fixture values that subject needs, so a fixture is read beside the
// component it is about rather than four hundred lines above it. The
// order below is the order a reader meets them in, and `gallery/case.tsx`
// holds the one wrapper they all draw into.
//
// **The hints lead on purpose.** A hint opens above its control and
// flips below when the window has no room above, so the one fixture
// that can exercise the flip is the one nearest the top of the window;
// everything after it is ordered for a person reading down the page.

import { useSay } from "../ui";
import { Conversation } from "./gallery/conversation";
import { Filed } from "./gallery/filed";
import { Hints } from "./gallery/hints";
import { Parts } from "./gallery/parts";
import { Presences } from "./gallery/presence";
import { Produced } from "./gallery/produced";
import { Screens } from "./gallery/screens";
import { Shelved } from "./gallery/shelved";
import { Switches } from "./gallery/switches";

export function Gallery() {
  const say = useSay();
  return (
    <div class="mx-auto w-full max-w-talk px-pane py-pane">
      <h1 class="mb-wide text-heading text-text">{say("gallery_title")}</h1>
      <Hints />
      <Presences />
      <Conversation />
      <Produced />
      <Filed />
      <Screens />
      <Shelved />
      <Parts />
      <Switches />
    </div>
  );
}
