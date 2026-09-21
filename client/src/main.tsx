// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mount point, and nothing else: the address bar and the socket
// address are read here once and handed in, so every module below
// takes them as parameters rather than reaching for `window`.
//
// What the person prefers is reached the same way, through the one
// door `core/prefs.ts` opens. This file asks for that door once and
// hands the same one to the shell and to the face the page is drawn
// in, so a browser that offers no storage leaves both of them reading
// the one record that stands in for it.

import { createSignal } from "solid-js";
import { render } from "solid-js/web";

import { App } from "./app";
import { preferences } from "./core/prefs";
import { applyAppearance, watchMachineLighting } from "./views/setup/appearance";
import { langOf } from "./core/lang";
import { openConnection, socketUrl, tokenIn } from "./core/socket";
import { UiProvider } from "./ui";
import type { Effort } from "./wire";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  const prefs = preferences();
  // Before the first paint: a face chosen once is the face the next
  // window opens with, and applying it after mount is a visible change
  // of shape a person did not ask for.
  applyAppearance(document.documentElement, prefs.held().appearance);
  watchMachineLighting(document.documentElement, prefs);
  // Read once: the socket greets with it and the HTTP doors carry it
  // as a bearer header, and a second read could answer differently
  // after the address bar changed.
  const token = tokenIn(window.location.search);
  const conn = openConnection(
    socketUrl(window.location),
    token,
    langOf(navigator.language),
  );
  render(() => {
    // How hard the next dispatch asks the model to think, held for as
    // long as this page is open and written nowhere: the standing
    // answer is the city's `[model] effort`, and a copy kept in this
    // browser would silently outrank it.
    const [effort, chooseEffort] = createSignal<Effort | null>(null);
    return (
      <UiProvider
        value={{
          conn,
          prefs,
          effort,
          chooseEffort,
          bar: window.location,
          origin: window.location.origin,
          pairing: token,
          now: () => Date.now(),
        }}
      >
        <App />
      </UiProvider>
    );
  }, main);
}
