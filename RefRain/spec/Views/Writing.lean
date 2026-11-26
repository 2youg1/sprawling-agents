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
selection context action and the editor's attachment action use the same range.

The file canvas seats one Markdown header above one editing surface. Its switch
selects live or source without replacing the document. The hollow mark denotes
a draft; the tick denotes the acknowledged version. Refusal and uncertainty
have spoken and visible text. The lower-left readout has two lines: draft and
active logical-line bytes, then nine hexadecimal characters of the saved
BLAKE3 and an ellipsis. Activation copies all 64 characters. The readout sits
outside the writing measure or follows the surface where the margin is too
small. File category and link stay reachable through the editor's context tools.

Writing is a top-level settings entry between Running and Extensions. The
browser keeps its measure, typewriter preference and paragraph indentation.
Gold uses the short golden share, silver uses shell lines 4 and 10, and wide
uses lines 3 and 11. The available measure caps all three; narrow writing is
one column. The gutter's trailing rule is the writing edge. The active
selection head determines the emphasized logical line.

Typewriter adjustment is for input and a single keyboard caret crossing
visual rows. Within a row, single pointer placement, composing, selection
extension and drag/drop use ordinary editor visibility. Adjustment moves only
the distance to the nearest edge of the middle half of the editor viewport.

Indentation is off, presentation, or source input. Presentation indents only
the first visual line of nonempty top-level prose paragraphs in the live
reading. Headings, lists, tables, code fences including Mermaid, quotes and
blank lines receive no indentation. Source input
completes up to two leading ideographic spaces when typing starts a new
paragraph. The most recent generated prefix is withdrawn if that paragraph
becomes a structural block during editing; handwritten characters remain.
Existing other leading whitespace is kept. Pasting, moving text
and opening a file are independent of source indentation. An explicit context
action previews the affected paragraphs and completes their prefix in one undo
step. It never appends indentation to an already completed paragraph.

Context tools open with or without a selection. Inline format and attachment
actions use their original ranges. Quote covers selected logical lines or the
caret's current line,
adds a marker to unquoted lines, and removes one layer if the nonblank lines
are already quoted. One transaction covers blank lines inside the selection.
Search shares one floating panel between the
context menu and Accel-F, keeps the existing query options and commands, and
restores editor focus when dismissed. Enter and Shift-Enter advance in the
query, Escape dismisses. A read-only editor can search and cannot replace.

The undo pool lists up to 64 recent actions in the open editor, bounded by
4,194,304 recorded UTF-16 code units of insertions and removals. Selection and preview do
not edit. Applying multiple actions is one CodeMirror transaction and one
ordinary undo step. Each inverse rebases through later retained changes in
reverse order. Overlap requires selecting the later dependency or preserving the
original document. A reversed dependency that has restored the exact bytes
allows an earlier edit to be selected separately. A document change between preview and application requires
the plan to be taken again from the current editor state. A received change
without a local edit history starts a new pool; an acknowledged save with the
same editor text does not.

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

inductive IndentMode where
  | off
  | display
  | source
  deriving DecidableEq, Repr

def missingIndent (leading : Nat) : Nat := 2 - leading

theorem finishedIndentIsStable (leading : Nat) (finished : 2 ≤ leading) :
    missingIndent leading = 0 := by
  exact Nat.sub_eq_zero_of_le finished

def typewriterOffset (top bottom caret : Int) : Int :=
  if caret < top then caret - top else if bottom < caret then caret - bottom else 0

theorem insideBandStays (top bottom caret : Int) (low : top ≤ caret) (high : caret ≤ bottom) :
    typewriterOffset top bottom caret = 0 := by
  simp [typewriterOffset, Int.not_lt.mpr low, Int.not_lt.mpr high]

def mayApplyUndo (selection : List Nat) (unselectedOverlap : Bool) : Bool :=
  !selection.isEmpty && !unselectedOverlap

theorem overlapBlocksUndo (selection : List Nat) : mayApplyUndo selection true = false := by
  simp [mayApplyUndo]

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
