// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal } from "solid-js";
import { onCleanup as onDestroy } from "solid-js";
import { get } from "../../core/store";
import { fill, say } from "../../core/lang";
import { ui } from "../../ui";
import type { ExportFormat, Query, UsageKind } from "../../wire";
import { saveFile } from "../editor/saved_file";
import { exportedOf, FORMATS, TYPES } from "./usage_export";
import type { Exported } from "./usage_export";
import Look from "./usage_export.look.solid";
interface Props {
  readonly what: UsageKind;
}
import { storeValue } from "../../core/solid_store";

export default function View(props: Props) {
  const u = ui();
  const lang = u.lang;
  const $lang = storeValue(lang);
  const [waiting, __rrSet0] = createSignal<ExportFormat | null>(null);
  const [missed, __rrSet1] = createSignal<Exported | null>(null);
  const waits: Record<ExportFormat, (() => void) | undefined> = {
    jsonl: undefined,
    csv: undefined,
  };
  onDestroy(() => {
    for (const format of FORMATS) waits[format]?.();
  });
  function press(format: ExportFormat): void {
    const query: Query = { usage_export: { what: props.what, format } };
    __rrSet0(() => format);
    __rrSet1(() => null);
    const held = u.conn.asking.ask(query);

    waits[format]?.();
    const before = get(held);
    u.conn.asking.refresh(query);
    const off = held.subscribe((answer) => {
      if (answer === undefined || answer === before) return;
      const exported = exportedOf(answer);
      if (exported === undefined) return;
      if (exported.kind === "file")
        saveFile(`${props.what}-usage.${format}`, TYPES[format], exported.body);
      else __rrSet1(() => exported);
      __rrSet0(() => null);
      waits[format] = undefined;
      queueMicrotask(off);
    });
    waits[format] = off;
  }
  const unavailable = createMemo(() => {
    const result = missed();
    if (result?.kind !== "unavailable") return undefined;
    return {
      said: fill(say($lang(), "answer_unavailable"), { query: result.query }),
      reason: result.reason,
    };
  });
  return (
    <>
      <Look
        buttons={FORMATS.map((format) => ({
          format,
          label: say($lang(), format === "jsonl" ? "usage_export_jsonl" : "usage_export_csv"),
          loading: waiting() === format,
          press: () => {
            press(format);
          },
        }))}
        unavailable={unavailable()}
      />
    </>
  );
}
