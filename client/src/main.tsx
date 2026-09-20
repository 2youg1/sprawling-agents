// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mount point, and nothing else: the address bar and the socket
// address are read here once and handed in, so every module below
// takes them as parameters rather than reaching for `window`.
//
// Browser storage is reached the same way, through the one door
// `core/prefs.ts` opens. This file asks for those rows once and hands
// the same rows to the cache and to the face the page is drawn in, so
// a browser that offers no storage leaves both of them reading the
// one store that stands in for it.

import { render } from "solid-js/web";

import { App } from "./app";
import { browserRows, loadPrefs, readAppearance } from "./core/prefs";
import { applyAppearance, watchMachineLighting } from "./views/setup/appearance";
import { openConnection, socketUrl, tokenIn } from "./core/socket";
import { UiProvider } from "./ui";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  const rows = browserRows();
  const prefs = loadPrefs(rows, navigator.language);
  // Before the first paint: a face chosen once is the face the next
  // window opens with, and applying it after mount is a visible change
  // of shape a person did not ask for.
  applyAppearance(document.documentElement, readAppearance(rows));
  watchMachineLighting(document.documentElement, rows);
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
