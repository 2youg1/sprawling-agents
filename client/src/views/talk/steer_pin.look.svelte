<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The steer pin (refrain §3-5): a wedge pointing left at the place a
steer pressed now is heard - the call line the run is in, or the end of
the words it is still saying. It repeats what the coin's name already
says (client/Spec.lean §4-13), so it is drawn for the eye and hidden from
a reader.

Drawn with borders rather than a fill, so a forced-colours mode keeps the
shape when it repaints the ink. Its size is the status dot's: half of it
tall on each side of the point and three quarters of it long, so the pin
and the dot beside it read as one family of marks. -->
<span class="steer-pin" aria-hidden="true"></span>

<style>
  .steer-pin {
    display: inline-block;
    inline-size: 0;
    block-size: 0;
    border-block: calc(var(--spacing-dot) / 2) solid transparent;
    border-inline-end: calc(var(--spacing-dot) * 3 / 4) solid var(--color-accent);
    vertical-align: middle;
  }
</style>
