<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A call that took a screenshot (client/Spec.lean §4-45, §4-61). The browser
  // tool records the picture's place in the content store and its two
  // sides; the picture's bytes are fetched by that place, window by
  // window, and drawn at the picture's own proportions. Until they
  // arrive, and when they cannot, the frame is drawn empty and says
  // which - a blank reads as a picture of nothing.
</script>

<script lang="ts">
  import { storedObject } from "../../core/document_bytes";
  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { StoredBytes } from "../refrain/fetched.svelte";
  import type { Picture } from "./reading";

  interface Props {
    readonly picture: Picture;
  }

  const { picture }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const object = $derived(storedObject(picture.image));
  const fetched = new StoredBytes(u.conn.asking);

  $effect(() => {
    fetched.start(object);
  });

  // The picture as an address the image element reads, made once its
  // bytes are whole and let go when they change or the view goes.
  let source = $state<string | null>(null);
  $effect(() => {
    const value = fetched.value;
    if (value?.kind !== "whole") return;
    const url = URL.createObjectURL(new Blob([value.bytes], { type: picture.media_type }));
    source = url;
    return () => {
      URL.revokeObjectURL(url);
      source = null;
    };
  });

  const said = $derived(
    object === null || fetched.lost !== null || fetched.value?.kind === "too_large" ? "inspect_shot_lost" : "inspect_shot_reading",
  );
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
    {#if source !== null}
      <img
        class="block max-h-full max-w-full rounded-card border border-edge object-contain"
        src={source}
        width={picture.width}
        height={picture.height}
        alt={fill(say($lang, "inspect_shot"), {
          width: String(picture.width),
          height: String(picture.height),
          media: picture.media_type,
        })}
        style:aspect-ratio="{picture.width} / {picture.height}"
      />
    {:else}
      <div
        class="flex items-center justify-center rounded-card border border-dashed border-edge-input"
        style:aspect-ratio="{picture.width} / {picture.height}"
        style:width="min(100cqw, calc(100cqh * {picture.width} / {picture.height}))"
      >
        <span class="px-pane text-center text-note text-text-faint">{say($lang, said)}</span>
      </div>
    {/if}
  </div>
  <p class="shrink-0 font-mono text-note break-all text-text-quiet select-all">{picture.image}</p>
</div>
