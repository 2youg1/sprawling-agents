// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId } from "solid-js";
import type { Block, Inline } from "../../wire";
const words = (text: string): Inline => ({ text });
const formula = (source: string): Inline => ({ unsupported: { construct: "math", source } });
const span = { start: 0, end: 0 };
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
  {
    paragraph: {
      span,
      inline: [
        words("A command KaTeX does not know stays as written: "),
        formula("$\\notacommand{x}$"),
      ],
    },
  },
];
import { say } from "../../core/lang";
import { ui } from "../../ui";
import { rowsLookOf } from "../palette/rows";
import { PaletteRows } from "../palette/rows_view";
import Laid from "../editor/laid.solid";
import { GalleryCase } from "./case";
import { storeValue } from "../../core/solid_store";

export default function FormulaAndPaletteCases() {
  const lang = storeValue(ui().lang);
  const ignore = (): void => undefined;
  const id = createUniqueId();
  const [cursor, setCursor] = createSignal(0);
  const ENTRIES = createMemo(() => [
    { label: say(lang(), "palette_transcribe"), hint: "hall/mayor", act: ignore },
    { label: say(lang(), "city_stop"), hint: "halt", act: ignore },
    { label: "lab", hint: say(lang(), "palette_building"), act: ignore },
  ]);
  return (
    <>
      <GalleryCase label="reply · inline and display formulas, and one KaTeX refuses">
        <div class="text-body">
          <Laid blocks={BLOCKS} />
        </div>
      </GalleryCase>
      <GalleryCase label="palette · the transcription entry on a conversation page">
        <PaletteRows
          look={rowsLookOf(
            { listing: { kind: "places" }, shown: ENTRIES(), cursor: cursor(), id, lang: lang() },
            { hover: setCursor, pick: (item) => item.act() },
          )}
        />
      </GalleryCase>
    </>
  );
}
