<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Formulas as a reply or a document preview draws them (client/Spec.lean
  // §4-64a): an inline formula inside a sentence, a display formula on a
  // line of its own, and one KaTeX refuses, which stays its source. And
  // the palette's list on a conversation page where the city
  // transcribes, with the entry that speaks into the box (4-64b).
  import type { Block, Inline } from "../../wire";

  const words = (text: string): Inline => ({ text });
  const formula = (source: string): Inline => ({ unsupported: { construct: "math", source } });
  const span = { start: 0, end: 0 };

  // wording-ok: fixture text is a model's reply, not a page's words
  const BLOCKS: readonly Block[] = [
    {
      paragraph: {
        span,
        inline: [
          words("The area of a circle is "),
          formula("$\\pi r^2$"),
          words(", and its circumference "),
          formula("$2\\pi r$"),
          words("."),
        ],
      },
    },
    { paragraph: { span, inline: [formula("$$\\sum_{k=1}^{n} k = \\frac{n(n+1)}{2}$$")] } },
    // wording-ok: fixture text is a model's reply, not a page's words
    { paragraph: { span, inline: [words("A command KaTeX does not know stays as written: "), formula("$\\notacommand{x}$")] } },
  ];
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Entry } from "../palette/entry";
  import Rows from "../palette/rows.svelte";
  import Laid from "../refrain/laid.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
  const ignore = (): void => undefined;

  const ENTRIES = $derived<readonly Entry[]>([
    { label: say($lang, "palette_transcribe"), hint: "hall/mayor", act: ignore },
    { label: say($lang, "city_stop"), hint: "halt", act: ignore },
    { label: "lab", hint: say($lang, "palette_building"), act: ignore },
  ]);
</script>

<Case label="reply · inline and display formulas, and one KaTeX refuses">
  <div class="text-body"><Laid blocks={BLOCKS} /></div>
</Case>

<Case label="palette · the transcription entry on a conversation page">
  <Rows listing={{ kind: "places" }} shown={ENTRIES} cursor={0} onHover={ignore} onPick={ignore} />
</Case>
