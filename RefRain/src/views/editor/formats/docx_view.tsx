// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, Switch } from "solid-js";
import { fill, say, type Key } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { count, kib } from "../../../core/time";
import { ui } from "../../../ui";
import type { B3Hash } from "../../../wire";
import type { Read } from "./docx";
import { NAME, TOOL } from "./format";
import Frame from "./frame.solid";
import Label from "./label.solid";
import { UNPACKED_MAX } from "./zip";

export default function Docx(props: {
  readonly bytes: Uint8Array;
  readonly version: B3Hash | null;
  readonly name: string;
}) {
  const lang = storeValue(ui().lang);
  const [read, setRead] = createSignal<Read | null>(null);
  createEffect(
    () => props.bytes,
    (bytes) => {
      let live = true;
      setRead(null);
      void import("./docx")
        .then(({ readDocx }) => readDocx(bytes))
        .then(
          (result) => {
            if (live) setRead(result);
          },
          () => {
            if (live) setRead({ kind: "broken", reason: say(lang(), "refrain_preview_failed") });
          },
        );
      return () => {
        live = false;
      };
    },
  );
  function counted(n: number, one: Key, many: Key): readonly string[] {
    if (n === 0) return [];
    return [n === 1 ? say(lang(), one) : fill(say(lang(), many), { n: count(n) })];
  }
  const drawn = createMemo(() => {
    const held = read();
    return held?.kind === "drawn" ? held : undefined;
  });
  const notes = createMemo(() => {
    const coverage = drawn()?.coverage;
    return coverage === undefined
      ? []
      : [
          ...counted(coverage.revisions, "format_docx_revisions_one", "format_docx_revisions"),
          ...counted(coverage.comments, "format_docx_comments_one", "format_docx_comments"),
          ...counted(coverage.objects, "format_docx_objects_one", "format_docx_objects"),
        ];
  });
  const refusal = createMemo(() => {
    const held = read();
    if (held === null || held.kind === "drawn") return "";
    const reason =
      held.kind === "broken"
        ? held.reason
        : held.why === "too_large"
          ? fill(say(lang(), "format_zip_too_large"), { limit: kib(UNPACKED_MAX) })
          : say(lang(), "format_zip_shape");
    return fill(say(lang(), "format_broken"), { format: NAME.docx, reason });
  });
  return (
    <div class="flex min-h-0 flex-1 flex-col">
      <Label
        format="docx"
        tool={TOOL.docx}
        settings={[say(lang(), "format_docx_settings")]}
        version={props.version}
        notes={notes()}
      />
      <Switch fallback={<p class="px-wide py-snug text-note text-text-quiet">{refusal()}</p>}>
        <Match when={read() === null}>
          <p class="px-wide py-snug text-note text-text-faint">
            {fill(say(lang(), "format_reading"), { format: NAME.docx })}
          </p>
        </Match>
        <Match when={drawn()}>
          {(page) => (
            <div class="flex min-h-0 flex-1 flex-col bg-raised">
              <Frame page={page().page} scheme="built" title={props.name} />
            </div>
          )}
        </Match>
      </Switch>
    </div>
  );
}
