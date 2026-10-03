-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.Acceptance.Collaboration

/-!
# The whole walk: three servings of one folder, in order.

* **the first serving** — the first day;
* **the second serving** — the crash;
* **the third serving** — the morning after, then the collaboration, which
  stands on the city the morning left.

The steps of each part are in `Sprawling.Acceptance.Walk` and
`Sprawling.Acceptance.Collaboration`; this module owns only their order, which
the walk and the person's checklist both read.
-/

namespace Sprawling.Acceptance

open Sprawling

/-- The steps the third serving takes, in its order. -/
def thirdServing (setting : Setting) : List Step :=
  morningAfter setting ++ collaboration setting

/-- The name of every step the walk takes, in its order. -/
def walkedSteps (setting : Setting) : List String :=
  (firstDay setting).map (·.name) ++ [interruptedStep] ++ (thirdServing setting).map (·.name)

/-- Walks one fresh folder through all three servings, asks the city that came
back `atLast` once every step has held, and throws the folder away. -/
def walk (setting : Setting) (atLast : Ground → IO α) : IO α := do
  let stage ← Stage.raise setting.door
  try
    mountShelf stage.city setting.shelf
    stage.serving setting.door .closedInOrder fun ground => Step.runAll ground (firstDay setting)
    interrupted setting stage
    stage.serving setting.door .closedInOrder fun ground => do
      Step.runAll ground (thirdServing setting)
      atLast ground
  finally
    stage.discard

end Sprawling.Acceptance
