-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Frame
import Sprawling.Door
import Sprawling.Ground
import Sprawling.Check
import Sprawling.Model
import Sprawling.Provider
import Sprawling.Layer
import Sprawling.Person
import Sprawling.Regression
import Sprawling.Acceptance.Script
import Sprawling.Acceptance.Stage
import Sprawling.Acceptance.Walk
import Sprawling.Acceptance.Collaboration
import Sprawling.Acceptance.Servings
import Sprawling.Acceptance.Checklist

/-! The library index. It holds no logic; every rule lives in the module that
owns it, and the dependency order is the one `tools/adversary/Spec.lean` section 7
draws: `Model` → `Door` → `Frame`, `Model` → `Ground` → `Door`, `Provider` →
`Ground` for the second world, `Layer` → `Ground` with `Check` for the third,
`Person` on `Layer` for the fourth, and the acceptance world under
`Sprawling.Acceptance` on `Provider` and `Layer`. The checker imports no
specification: the chain and snapshot theorems it cites in comments live in
`crates/kernel/spec/Ledger.lean` and `crates/storage/spec/Snapshot.lean`. -/
