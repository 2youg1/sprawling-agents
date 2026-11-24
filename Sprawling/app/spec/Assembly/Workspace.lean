-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
## 8-160 Local workspace files

A registered directory is the original directory selected by the person. The local service keeps a versioned catalogue in the city’s protected state. Each record has its own identity, category, reading position and last-opened time. The directory ID and file record ID are independent of city and session addresses. Copying the directory into a building would sever the relationship with changes made in the file manager, so workspace operations use the registered directory directly.

A directory capability confines browsing and reads. The root is pinned to its observed filesystem identity. Registered roots may neither contain the city nor stand inside it, and reserved metadata components, symbolic links, junctions and hard-linked documents are excluded. Listing opens only the requested directory, while record lookup after a move may search the workspace. Native notifications for a bounded set of opened directories invalidate their listing snapshots without periodic enumeration. The client refreshes on foreground return, and a missed notification leaves reference and save version checks in force. A same-volume move with evidence of the same file preserves its record. A copy gets a separate record. An unproven replacement preserves the former record and requires explicit relinking to inherit its metadata.

A content version is the BLAKE3 digest of the bytes. A save checks the record’s filesystem identity and exact baseline in the same pinned parent used for the atomic rename. RefRain serializes its saves. The filesystem orders a third-party write that occurs concurrently with the final comparison and rename. A journal ties the staged file’s identity and new digest to its record so recovery does not infer identity from content similarity. A save with a stale baseline leaves the file bytes and the prior record intact. A new file at an old record’s path never silently inherits an old citation. The checked save door admits Markdown and plain-text/code extensions, while PDF, images, HTML and Office documents are read-only.

The catalogue schema is numbered. A refused catalogue read or a failed metadata write does not substitute an empty catalogue for an existing one. `cargo test -p sprawling workspace` exercises the real directory files, including record recovery following a move, independent copy progress and an external edit during review.
-/

namespace Sprawling.Assembly.Workspace

inductive Save where
  | replace
  | conflict
  deriving DecidableEq

def decideSave {Identity Version : Type} [DecidableEq Identity] [DecidableEq Version]
    (expected actual : Identity) (baseline current : Version) : Save :=
  if expected = actual ∧ baseline = current then .replace else .conflict

theorem changed_content_refuses {Identity Version : Type} [DecidableEq Identity]
    [DecidableEq Version] (expected actual : Identity) (baseline current : Version)
    (changed : baseline ≠ current) : decideSave expected actual baseline current = .conflict := by
  simp [decideSave, changed]

end Sprawling.Assembly.Workspace
