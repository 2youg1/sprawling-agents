-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Registered workspace writing

The source and live writing readings share one document, selection, scroll
anchor and undo history. Source uses its own theme role for Markdown syntax.
Live rendering leaves every block touched by a selection in source. While
composition is active, a requested reading is applied after composition ends.
Appearance changes never write the document. Native context menu keys, a
selection context action and the editor toolbar attach the same selection.

A settled reply can offer a replacement for an attached Markdown range.
Discussion has no write action. The proposal opens on the document canvas and
shows the quoted source and replacement. Review may reveal the source, amend
the replacement, accept, or reject. Only acceptance can issue the workspace
write. The workspace checks the same baseline immediately before replacing
bytes. After refusal the proposal and user's edits remain. A moved file still
uses its registered identity. If the version, range or quotation disagrees,
review keeps the proposal and disables acceptance.

The editor uses normalized UTF-16 offsets at the browser boundary, and converts
the accepted range to immutable baseline bytes. Every untouched byte remains
original, including BOM and mixed line endings. New lines use the first line
ending read from the baseline. A received write rebases disjoint unsaved edits
onto the new file version. Overlapping local edits need review before acceptance.

Conformance uses the workspace file canvas, the actual reply reader, and a
version mismatch between proposal generation and acceptance. Test that keyboard
undo and selected text survive switching writing readings and subjects; compose
CJK text while changing readings; attach one passage and complete discussion
and rewrite separately. Verify bytes before review, after decline and after
acceptance, and keep the replacement draft after a baseline refusal.
-/

namespace Client.Views.Writing

inductive Decision where
  | read
  | reject
  | accept
  deriving DecidableEq, Repr

structure BaselineCheck where
  versionMatches : Bool
  quoteMatches : Bool
  localOverlap : Bool

def mayIssueWrite (decision : Decision) (check : BaselineCheck) : Bool :=
  decision = .accept && check.versionMatches && check.quoteMatches && !check.localOverlap

theorem readDoesNotWrite (check : BaselineCheck) : mayIssueWrite .read check = false := by
  simp [mayIssueWrite]

theorem rejectDoesNotWrite (check : BaselineCheck) : mayIssueWrite .reject check = false := by
  simp [mayIssueWrite]

theorem changedOriginBlocksAccept (check : BaselineCheck) (changed : check.versionMatches = false) :
    mayIssueWrite .accept check = false := by
  simp [mayIssueWrite, changed]

end Client.Views.Writing
