<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The reading face (wire D52): the type cards of the appearance group
  // with Libron chosen, and the reading text Libron is for - a settled
  // reply laid by the city, the same reply still drawn as it arrived,
  // and the person's own message - with Chinese and English in one
  // paragraph, the case a serif stack ending on the generic `serif` has
  // to hold. Each case carries `data-read` itself, so the page around it
  // keeps whatever face the person chose.
  import type { Appearance } from "../../core/appearance";
  import type { Block, Inline } from "../../wire";
  import { RunId } from "../../wire";

  const LIBRON: Appearance = {
    lighting: "system",
    sans: "geist",
    mono: "geist",
    sansStack: "",
    monoStack: "",
    reading: "libron",
    body: null,
    density: "comfortable",
    chroma: "full",
    motion: "system",
    glass: "on",
    blend: null,
  };

  const words = (text: string): Inline => ({ text });
  const span = { start: 0, end: 0 };

  // wording-ok: fixture text is a model's reply, not a page's words
  const HEADING = "账本里的一行";
  // wording-ok: fixture text is a model's reply, not a page's words
  const OPENING = "这座城只有一本账：每一次 run 都从它读出，也只往它的末尾写。The Ledger is append-only, so ";
  // wording-ok: fixture text is a model's reply, not a page's words
  const CLOSING = " replays the whole city from its first line, and 中文与 English 在同一行里各用设备自带的衬线字体。";

  const BLOCKS: readonly Block[] = [
    { heading: { span, level: 2, inline: [words(HEADING)] } },
    {
      paragraph: {
        span,
        inline: [words(OPENING), { code: "sprawling view --records" }, words(CLOSING)],
      },
    },
    // wording-ok: fixture text is a model's reply, not a page's words
    { paragraph: { span, inline: [{ strong: [words("Code stays mono.")] }, words(" 代码、路径与命令仍是等宽字体。")] } },
    { code: { span, info: "sh", text: "sprawling view tree --since 2026-10-01" } },
  ];

  // wording-ok: fixture text is a reply that has not been laid yet
  const ARRIVED = "一段还没被城读过的回复照原样画出。A reply the city has not laid yet is drawn as it arrived, in the reading face all the same.";
  // wording-ok: fixture text is the person's own message
  const ASKED = "请把这一段改得更短一些。Keep the second paragraph, and cut the rest by half.";

  const RUN = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e10");
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Prose from "../prose.svelte";
  import Laid from "../refrain/laid.svelte";
  import AppearanceType from "../setup/appearance_type.svelte";
  import Person from "../talk/person.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const ignore = (): void => undefined;
</script>

{#snippet noFoot()}{/snippet}

<Case label="reading face · the type cards with Libron chosen" width={1280}>
  <div class="grid grid-fit items-start gap-base">
    <AppearanceType look={LIBRON} write={ignore} foot={noFoot} />
  </div>
</Case>

<Case label="reading face · a settled reply in Libron, Chinese and English in one paragraph">
  <div class="text-body" data-read="libron"><Laid blocks={BLOCKS} /></div>
</Case>

<Case label="reading face · a reply drawn as it arrived, and the person's own message, in Libron">
  <div class="flex flex-col text-body" data-read="libron">
    <Prose text={ARRIVED} />
    <Person text={ASKED} label={say($lang, "talk_you")} at={undefined} entry={null} run={RUN} onHover={ignore} />
  </div>
</Case>
