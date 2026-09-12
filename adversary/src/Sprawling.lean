-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Frame
import Sprawling.Door
import Sprawling.Ground
import Sprawling.Check
import Sprawling.Model
import Sprawling.Regression

/-! The library index. It holds no logic; every rule lives in the module that
owns it, and the dependency order is the one `adversary-SPEC.md` section 7
draws: `Model` → `Door` → `Frame`, `Model` → `Ground` → `Door`. -/
