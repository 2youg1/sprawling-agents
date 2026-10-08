// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mount point, and nothing else: the address bar and the line the
// link speaks through are read here once and handed in, so every module below
// takes them as parameters rather than reaching for `window`.
//
// What the person prefers is reached the same way, through the one
// door `core/prefs.ts` opens. This file asks for that door once and
// hands the same one to the shell, so a browser that offers no storage
// leaves the page reading the one record that stands in for it.
//
// Before the shell mounts, the page comes in (`core/local/entering.ts`):
// with the open code the address's fragment carries, with the device
// key this origin kept, or through the pairing page. The address bar
// carries no credential at any point.

import { derived, get } from "svelte/store";
import { mount, unmount } from "svelte";

import App from "./app.svelte";
import { langOf } from "./core/lang";
import { NO_CREDENTIAL, browserLabel, enter, mayHaveDoor, openCodeIn, pairWith, withoutOpenCode } from "./core/local/entering";
import type { Credential, Entry } from "./core/local/entering";
import { preferences } from "./core/prefs";
import type { PreferenceDoor } from "./core/prefs";
import { keepWithCity } from "./core/prefs_city";
import { dialFor } from "./core/remote/session";
import { openConnection } from "./core/socket";
import { timeAtRoot } from "./core/timing";
import type { Opening } from "./ui";
import Pairing from "./views/pairing.svelte";
import { applyAppearance, watchMachineLighting } from "./views/setup/appearance";
import { documentStage, wearKeptTheme } from "./views/setup/colours";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  // Read and taken off the address before anything else runs, so no
  // later read, history entry or screenshot of the address bar holds it.
  const open = openCodeIn(window.location.hash);
  if (open !== null) window.history.replaceState(null, "", withoutOpenCode(window.location));
  const prefs = preferences();
  // Before the first paint: a face chosen once is the face the next
  // window opens with, and applying it after mount is a visible change
  // of shape a person did not ask for.
  applyAppearance(document.documentElement, get(prefs.held).appearance);
  watchMachineLighting(document.documentElement, prefs);
  // The person's colours, laid over the built-in theme now and again
  // whenever they or the city's answer change them (roadmap CT).
  wearKeptTheme(documentStage(document), prefs);
  // The page's key interactions as User Timing entries, for whoever
  // measures the page; nothing reads them in a session of use.
  timeAtRoot(document);
  const origin = window.location.origin;
  const label = browserLabel(navigator.userAgent);
  const entering: Promise<Entry> = mayHaveDoor(window.location.protocol)
    ? enter(origin, open, label)
    : Promise.resolve({ kind: "enter", credential: NO_CREDENTIAL });
  void entering.then((entry) => {
    if (entry.kind === "enter") {
      openCity(main, prefs, entry.credential);
      return;
    }
    const page = mount(Pairing, {
      target: main,
      props: {
        why: entry.why,
        lang: derived(prefs.held, (held) => held.lang),
        label,
        pair: (code: string, named: string) => pairWith(origin, { code }, named),
        onPaired: (credential: Credential) => {
          void unmount(page).then(() => {
            openCity(main, prefs, credential);
          });
        },
      },
    });
  });
}

function openCity(target: HTMLElement, prefs: PreferenceDoor, credential: Credential): void {
  const opening: Opening = {
    // A device the city no longer knows comes back through the pairing
    // page, which a fresh load of the page draws.
    conn: openConnection(
      dialFor(window.location, credential, () => {
        window.location.reload();
      }),
      langOf(navigator.language),
    ),
    prefs,
    bar: window.location,
    origin: window.location.origin,
    credential: credential.current,
    now: () => Date.now(),
  };
  keepWithCity(prefs, opening.conn);
  mount(App, { target, props: { opening } });
}
