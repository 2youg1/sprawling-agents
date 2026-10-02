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

import { get } from "svelte/store";
import { mount } from "svelte";

import App from "./app.svelte";
import { langOf } from "./core/lang";
import { preferences } from "./core/prefs";
import { keepWithCity } from "./core/prefs_city";
import { dialFor } from "./core/remote/session";
import { openConnection, tokenIn } from "./core/socket";
import type { Opening } from "./ui";
import { applyAppearance, watchMachineLighting } from "./views/setup/appearance";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  const prefs = preferences();
  // Before the first paint: a face chosen once is the face the next
  // window opens with, and applying it after mount is a visible change
  // of shape a person did not ask for.
  applyAppearance(document.documentElement, get(prefs.held).appearance);
  watchMachineLighting(document.documentElement, prefs);
  // Read once: the socket greets with it and the HTTP doors carry it
  // as a bearer header, and a second read could answer differently
  // after the address bar changed.
  const token = tokenIn(window.location.search);
  const opening: Opening = {
    conn: openConnection(dialFor(window.location), token, langOf(navigator.language)),
    prefs,
    bar: window.location,
    origin: window.location.origin,
    pairing: token,
    now: () => Date.now(),
  };
  keepWithCity(prefs, opening.conn);
  mount(App, { target: main, props: { opening } });
}
