// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The mount point, and nothing else: the browser's globals are read here
// once and handed in, so every module below takes the address bar and
// the language as parameters rather than reaching for `window`.

import { render } from "solid-js/web";

import { App } from "./app";
import { langOf } from "./core/lang";
import "./theme.css";

const main = document.getElementById("main");
if (main !== null) {
  render(
    () => <App bar={window.location} lang={langOf(navigator.language)} />,
    main,
  );
}
