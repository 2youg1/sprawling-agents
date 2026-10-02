<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A call that took a screenshot (client/Spec.lean §4-45). The browser tool
  // records the picture's place in the content store and its two sides;
  // the wire has no answer that carries a picture's bytes to the page, so
  // this view draws the frame the picture would fill, at its own
  // proportions, and says where the city keeps it - rather than a blank
  // that reads as a picture of nothing.
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Picture } from "./reading";

  interface Props {
    readonly picture: Picture;
  }

  const { picture }: Props = $props();

  const lang = ui().lang;
</script>

<div class="flex min-h-0 flex-1 flex-col gap-base bg-page px-wide py-pane">
  <p class="shrink-0 text-note text-text-faint">
    {fill(say($lang, "inspect_shot"), {
      width: String(picture.width),
      height: String(picture.height),
      media: picture.media_type,
    })}
  </p>
  <!-- The frame keeps the picture's proportions inside whatever room the
  region has, the way an image that is contained would. -->
  <div class="flex min-h-0 flex-1 items-center justify-center [container-type:size]">
    <div
      class="flex items-center justify-center rounded-card border border-dashed border-edge-input"
      style:aspect-ratio="{picture.width} / {picture.height}"
      style:width="min(100cqw, calc(100cqh * {picture.width} / {picture.height}))"
    >
      <span class="px-pane text-center text-note text-text-faint">{say($lang, "inspect_shot_unserved")}</span>
    </div>
  </div>
  <p class="shrink-0 font-mono text-note break-all text-text-quiet select-all">{picture.image}</p>
</div>
