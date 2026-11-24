// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Stands } from "./sheet";

export const FACE =
  "m-auto hidden w-full max-w-measure flex-col gap-base rounded-panel bg-raised p-pane opacity-0 " +
  "shadow-sheet transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave " +
  "open:flex open:opacity-100 open:ease-arrive starting:open:opacity-0 still:transition-none " +
  "backdrop:bg-transparent backdrop:backdrop-brightness-50";

export const STANDS: Record<Stands, string> = {
  centre: "",
  "below-top": "mt-section",
  "in-flow": "mt-section static",
};
