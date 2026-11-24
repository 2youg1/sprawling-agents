-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Appearance on a device

D127. Settings → Appearance (in the global navigation layer in either mode)
contains Session wallpaper and reading brightness. Selection changes the preview
immediately, while Apply commits both values together to the browser's local
store. The success receipt follows the storage transaction. Dismissing the layer
or choosing Revert restores the saved look. A failed write preserves the preview
for retry and leaves the saved choice unchanged. The settings grid owns both
controls and their feedback. The Settings layer owns the return focus. The
native Solid card renders its controls in the card's own slot owner. Its
appearance-settings entry mounts that card once and disposes its subscription
on departure. Tests reopen the same browser profile after Apply and rebuild
the card from the stored result.

The wallpaper is solid, an offline marble image, or a local image. A local
picker that can grant a persistent file handle retains that handle, and startup
rechecks it without silently prompting. Refocusing the browser rechecks a
linked image. An unavailable file shows the solid
Session surface and keeps its name for recovery. Browsers with only a file
input keep a local image copy and show which source is saved. Files stay on the
device, distinct from city-owned preferences (4-29). The reading and editing
canvas is solid. PDF paper and ink reuse the rendered page, with the original
PDF pixels available on hold.

Brightness is a bounded integer percentage, with 50 used for a new browser
profile and saved values retained. The canvas offset adjusts OKLCH lightness.
The PDF default is cool stone with light ink and dark paper. At 0, 50 and 100
percent, paper lightness is 0.13, 0.17 and 0.23, and ink lightness is 0.8378,
0.844 and 0.8484. Adjacent ranges interpolate in OKLCH with a fixed 247 degree
hue and chroma of 0.023 for paper and 0.01656 for ink. The two endpoints of the
pixel mapping measure approximately −75 Lc at each reference brightness.
Forced colors retain system colors and suppress wallpaper. An opaque text
backing separates Session content from arbitrary colors in a user image.
The stored Blend opacity sets the temporary activity panel's background mix
in OKLCH. The panel text keeps its semantic ink color.

APCA observations use `apca-w3` 0.1.9 SA98G on sRGB after alpha composition,
with text first, surface second and signed Lc. Tier comparisons alone use the
magnitude. See the `apca-vectors.json` fixture and the browser and xtask tests
for reference values and low contrast cutoffs. WCAG 2.x checks retain their
established criteria. See CSS Color 4 for relative colors, missing components
and the shorter hue arc. A color that cannot be resolved or a backdrop that
cannot be known is reported as unmeasured.
-/

namespace Client.Views.Appearance

inductive Commit where
  | editing
  | saving
  | failed
  deriving DecidableEq, Repr

structure Preview (Choice : Type) where
  saved : Choice
  shown : Choice
  commit : Commit
  deriving Repr

def select (state : Preview Choice) (next : Choice) : Preview Choice :=
  if state.commit = .saving then state else { state with shown := next, commit := .editing }

def apply (state : Preview Choice) : Preview Choice :=
  { state with commit := .saving }

def settled (state : Preview Choice) (succeeded : Bool) : Preview Choice :=
  if state.commit = .saving then
    if succeeded then { saved := state.shown, shown := state.shown, commit := .editing }
    else { state with commit := .failed }
  else state

def revert (state : Preview Choice) : Preview Choice :=
  select state state.saved

theorem selection_preserves_saved (state : Preview Choice) (next : Choice) :
    (select state next).saved = state.saved := by simp [select]; split <;> rfl

theorem failed_apply_preserves_saved (state : Preview Choice) :
    (settled (apply state) false).saved = state.saved := by simp [apply, settled]

theorem committed_preview_becomes_saved (state : Preview Choice) :
    (settled (apply state) true).saved = state.shown := by simp [apply, settled]

end Client.Views.Appearance
