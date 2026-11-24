// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, Switch } from "solid-js";
import { fill, say, type Lang } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { B3Hash } from "../../../wire";
import Button from "../../parts/button.solid";
import EditorHost from "../editor_host";
import { phrasesIn, short } from "../reading";
import { saveFile } from "../saved_file";
import { NAME, TOOL } from "./format";
import Label from "./label.solid";

interface Side {
  readonly version: B3Hash;
  readonly bytes: Uint8Array;
}

interface Yield {
  readonly text: string;
  readonly silent: readonly number[];
}

async function read(format: "pdf" | "docx", lang: Lang, bytes: Uint8Array): Promise<Yield | null> {
  if (format === "docx") {
    const { docxLines } = await import("./docx");
    const lines = await docxLines(bytes);
    return lines === null ? null : { text: lines.join("\n"), silent: [] };
  }
  const { openPdf, pagesText } = await import("./pdf");
  const opening = openPdf(bytes);
  return opening.opened
    .then(async (opened) => {
      if (opened.kind !== "open") return null;
      const pages = await pagesText(opened.pdf);
      return {
        text: pages
          .map(
            (page, index) =>
              `— ${fill(say(lang, "format_page_mark"), { n: String(index + 1) })} —\n${page}`,
          )
          .join("\n"),
        silent: pages.flatMap((page, index) => (page.trim() === "" ? [index + 1] : [])),
      };
    })
    .finally(opening.close);
}

export default function Compare(props: {
  readonly format: "pdf" | "docx";
  readonly from: Side;
  readonly to: Side;
  readonly name: string;
}) {
  const lang = storeValue(ui().lang);
  const [texts, setTexts] = createSignal<
    { readonly from: Yield; readonly to: Yield } | "broken" | null
  >(null);
  const [host, setHost] = createSignal<HTMLDivElement>();
  createEffect(
    () => ({ format: props.format, a: props.from.bytes, b: props.to.bytes, lang: lang() }),
    ({ format, a, b, lang }) => {
      let live = true;
      setTexts(null);
      void Promise.all([read(format, lang, a), read(format, lang, b)]).then(
        ([from, to]) => {
          if (live) setTexts(from === null || to === null ? "broken" : { from, to });
        },
        () => {
          if (live) setTexts("broken");
        },
      );
      return () => {
        live = false;
      };
    },
  );
  createEffect(
    () => ({ parent: host(), held: texts(), name: props.name, phrases: phrasesIn(lang()) }),
    ({ parent, held, name, phrases }) => {
      if (parent === undefined || held === null || held === "broken") return;
      let opened: { readonly destroy: () => void } | undefined;
      let live = true;
      void import("../editing").then(({ openComparison }) => {
        if (live)
          opened = openComparison(
            parent,
            { from: held.from.text, to: held.to.text },
            name,
            phrases,
          );
      });
      return () => {
        live = false;
        opened?.destroy();
      };
    },
  );
  const notes = createMemo(() => {
    const versions = fill(say(lang(), "format_compared_versions"), {
      from: short(props.from.version),
      to: short(props.to.version),
    });
    const held = texts();
    if (held === null || held === "broken") return [versions];
    const silent = [...new Set([...held.from.silent, ...held.to.silent])].sort((a, b) => a - b);
    return silent.length === 0
      ? [versions]
      : [versions, fill(say(lang(), "format_no_text"), { pages: silent.join(", ") })];
  });
  const settings = () =>
    say(lang(), props.format === "pdf" ? "format_compare_pdf" : "format_compare_docx");
  function save(): void {
    const held = texts();
    if (held === null || held === "broken") return;
    const name = props.name;
    const comparison = {
      name,
      from: { label: short(props.from.version), text: held.from.text },
      to: { label: short(props.to.version), text: held.to.text },
      about: [`${NAME[props.format]} · ${TOOL[props.format]} · ${settings()}`, ...notes()],
    };
    void import("./compared").then(({ comparedMarkdown, comparedName }) => {
      saveFile(comparedName(name), "text/markdown", comparedMarkdown(lang(), comparison));
    });
  }
  const same = createMemo(() => {
    const held = texts();
    return held !== null && held !== "broken" && held.from.text === held.to.text;
  });
  return (
    <div class="refrain flex min-h-0 flex-1 flex-col">
      <Label
        format={props.format}
        tool={TOOL[props.format]}
        settings={[settings()]}
        version={null}
        notes={notes()}
      />
      <Switch
        fallback={
          <>
            <div class="flex justify-end border-b border-edge px-wide py-tight">
              <Button
                label={say(lang(), "refrain_export_comparison")}
                tone="quiet"
                onPress={save}
              />
            </div>
            <EditorHost ref={(node) => setHost(node)} />
          </>
        }
      >
        <Match when={texts() === null}>
          <p class="px-wide py-snug text-note text-text-faint">
            {fill(say(lang(), "format_reading"), { format: NAME[props.format] })}
          </p>
        </Match>
        <Match when={texts() === "broken"}>
          <p class="px-wide py-snug text-note text-text-quiet">
            {say(lang(), "format_compare_broken")}
          </p>
        </Match>
        <Match when={same()}>
          <p class="px-wide py-snug text-note text-text-quiet">
            {say(lang(), "format_compare_same")}
          </p>
        </Match>
      </Switch>
    </div>
  );
}
