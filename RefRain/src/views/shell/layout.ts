// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Navigation, NavigationLayer, ShellVisit, Subject } from "../../core/shell_visit";

export interface ShellLayout {
  readonly subject: Subject;
  readonly navigation: Navigation;
  readonly draft: boolean;
  readonly split: boolean;
  readonly splitLine: number;
  readonly resizeSplit: (line: number) => void;
  readonly splitTop: number;
  readonly resizeSplitTop: (percentage: number) => void;
  readonly peeking: boolean;
  readonly visit: ShellVisit;
  readonly closeNavigation: (returnFocus?: boolean) => void;
  readonly navigationCovered: boolean;
  readonly selectNavigation: (layer: NavigationLayer) => void;
  readonly setDensity: () => void;
  readonly setPeek: (on: boolean) => void;
  readonly toggleDraft: () => void;
  readonly toggleSplit: () => void;
  readonly showDraft: () => void;
}
