// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { derived, get } from "svelte/store";
import { mount, unmount } from "svelte";

import App from "./app.svelte";
import { langOf } from "./core/lang";
import {
  NO_CREDENTIAL,
  browserLabel,
  enter,
  mayHaveDoor,
  openCodeIn,
  pairWith,
  withoutOpenCode,
} from "./core/local/entering";
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
  const open = openCodeIn(window.location.hash);
  if (open !== null) window.history.replaceState(null, "", withoutOpenCode(window.location));
  const prefs = preferences();
  applyAppearance(document.documentElement, get(prefs.held).appearance);
  watchMachineLighting(document.documentElement, prefs);
  wearKeptTheme(documentStage(document), prefs);
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
