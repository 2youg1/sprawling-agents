<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The button and the icon-only key in every state they can be in: the
  // four tones, the two postures that answer "not now", and an icon key
  // that can be pressed beside one that says why it cannot.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import IconButton from "../../parts/icon_button.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<Case label="button · four tones">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" />
    <Button label={say($lang, "part_cancel")} tone="secondary" />
    <Button label={say($lang, "dismiss")} tone="quiet" />
    <Button label={say($lang, "part_delete")} tone="destructive" />
  </div>
</Case>

<Case label="button · loading and refused">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" loading />
    <Button label={say($lang, "part_save")} tone="primary" why={say($lang, "part_why_halted")} />
  </div>
</Case>

<!-- The key's name is its hint: hover or focus the first two to read
it; the third cannot be used and its hint says why. -->
<Case label="button · icon keys, live and refused">
  <div class="flex flex-wrap items-center gap-snug">
    <IconButton glyph="copy" label={say($lang, "code_copy")} />
    <IconButton glyph="cross" label={say($lang, "panel_close")} />
    <IconButton glyph="reveal" label={say($lang, "path_reveal")} why={say($lang, "part_why_halted")} />
  </div>
</Case>
