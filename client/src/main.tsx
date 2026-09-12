// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mount point, and nothing else: the browser's globals are read here
// once and handed in, so every module below takes the address bar, the
// storage and the socket address as parameters rather than reaching
// for `window`.

import { render } from "solid-js/web";

import { App } from "./app";
import { loadPrefs } from "./core/prefs";
import { applyStoredAppearance, watchMachineLighting } from "./views/setup/appearance";
import { openConnection, socketUrl, tokenIn } from "./core/socket";
import { UiProvider } from "./ui";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  const prefs = loadPrefs(window.localStorage, navigator.language);
  // Before the first paint: a face chosen once is the face the next
  // window opens with, and applying it after mount is a visible change
  // of shape a person did not ask for.
  applyStoredAppearance(document.documentElement, window.localStorage);
  watchMachineLighting(document.documentElement, window.localStorage);
  const conn = openConnection(socketUrl(window.location), tokenIn(window.location.search));
  render(
    () => (
      <UiProvider
        value={{
          conn,
          prefs,
          bar: window.location,
          origin: window.location.origin,
          now: () => Date.now(),
        }}
      >
        <App />
      </UiProvider>
    ),
    main,
  );
}
