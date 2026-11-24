// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say, type Key } from "../../core/lang";
import type { Keeper } from "../../core/prefs_city";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Badge from "../parts/badge.solid";

const WORDS: Record<Keeper, Key> = {
  browser: "setup_kept_browser",
  city: "setup_kept_city",
};

export default function Kept(props: { readonly keeper: Keeper }) {
  const lang = storeValue(ui().lang);
  return <Badge text={say(lang(), WORDS[props.keeper])} />;
}
