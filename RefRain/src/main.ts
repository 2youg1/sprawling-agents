// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { watchControlFeedback } from "./core/control_feedback";
import { derived, get } from "./core/store";
import { mountSolid } from "./views/solid_mount";

import App from "./app";
import { setUi } from "./ui";
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
import { wallpaperSettings } from "./core/wallpaper";
import type { PreferenceDoor } from "./core/prefs";
import { keepWithCity } from "./core/prefs_city";
import { dialFor } from "./core/remote/session";
import { openConnection } from "./core/socket";
import { timeAtRoot } from "./core/timing";
import type { Opening } from "./ui";
import Pairing, { PairingPending } from "./views/pairing.solid";
import { applyAppearance, watchMachineLighting } from "./views/setup/appearance";
import { documentStage, wearKeptTheme } from "./views/setup/colours";
import "./theme.css";

const root = document.getElementById("refrain-root");
if (root !== null) {
  watchControlFeedback(document);
  const open = openCodeIn(window.location.hash);
  if (open !== null) window.history.replaceState(null, "", withoutOpenCode(window.location));
  const prefs = preferences();
  applyAppearance(document.documentElement, get(prefs.held).appearance);
  watchMachineLighting(document.documentElement, prefs);
  wearKeptTheme(documentStage(document), prefs);
  wallpaperSettings();
  timeAtRoot(document);
  const origin = window.location.origin;
  const label = browserLabel(navigator.userAgent);
  const lang = derived(prefs.held, (held) => held.lang);
  const local = mayHaveDoor(window.location.protocol);
  const checking = local ? mountSolid(root, PairingPending, { lang }) : null;
  const entering: Promise<Entry> = local
    ? enter(origin, open, label).catch(() => ({
        kind: "pair",
        why: "unreachable",
        issue: { phase: open === null ? "challenge" : "pair", problem: "network" },
      }))
    : Promise.resolve({ kind: "enter", credential: NO_CREDENTIAL });
  void entering.then((entry) => {
    checking?.dispose();
    if (entry.kind === "enter") {
      openCity(root, prefs, entry.credential);
      return;
    }
    const page = mountSolid(root, Pairing, {
      why: entry.why,
      issue: entry.issue,
      check: entry.check,
      lang,
      label,
      pair: (code: string, named: string) => pairWith(origin, { code }, named),
      onPaired: (credential: Credential) => {
        page.dispose();
        openCity(root, prefs, credential);
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
  setUi(opening);
  mountSolid(target, App, {});
}
