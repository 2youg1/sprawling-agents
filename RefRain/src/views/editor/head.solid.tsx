// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, onCleanup, Show } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import Button from "../parts/button.solid";
import Segmented from "../parts/segmented.solid";
import { receiptWord, type HeadLook } from "./head";
import { short, type Reading } from "./reading";
import { saveFile } from "./saved_file";
import type { Session } from "./session";

interface HeadProps {
  readonly at: Address;
  readonly session: Session;
  readonly reading: Reading;
  readonly previewed: boolean;
  readonly live: boolean;
  readonly onPick: (reading: Reading) => void;
}

const READINGS: readonly { readonly value: Reading; readonly key: Key }[] = [
  { value: "source", key: "refrain_source" },
  { value: "live", key: "refrain_live" },
  { value: "preview", key: "refrain_preview" },
  { value: "diff", key: "refrain_diff" },
  { value: "versions", key: "refrain_versions" },
];

export default function Head(props: HeadProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const state = storeValue(() => props.session.state);
  const [exporting, setExporting] = createSignal<"asking" | "refused" | null>(null);
  let stopExport: (() => void) | undefined;
  onCleanup(() => stopExport?.());

  const cut = () => props.at.lastIndexOf("/") + 1;
  const name = () => props.at.slice(cut());
  const version = () => state()?.positions?.version;
  const said = () => receiptWord(state()?.receipt.kind ?? "clean");
  const options = () =>
    READINGS.filter(
      (each) =>
        (props.previewed || each.value !== "preview") && (props.live || each.value !== "live"),
    ).map((each) => ({ value: each.value, label: say(lang(), each.key) }));
  const blocked = (): { readonly why?: string } => {
    const held = state();
    if (held === undefined || held.locked !== null || held.positions === null)
      return { why: say(lang(), "refrain_save_locked") };
    const { kind } = held.receipt;
    if (kind === "saving" || kind === "pending") return { why: say(lang(), "refrain_save_wait") };
    return kind === "draft" || kind === "refused"
      ? {}
      : { why: say(lang(), "refrain_save_nothing") };
  };
  const markdown = () => {
    const file = state()?.file;
    return file?.kind === "text" && file.gathering.format === "markdown";
  };

  function exportHtml(): void {
    const standing = version();
    if (standing === undefined || exporting() === "asking") return;
    const at = props.at;
    const filename = name();
    setExporting("asking");
    let finished = false;
    const stop = u.conn.asking.ask({ export: { at, version: standing } }).subscribe((answer) => {
      const read = readAnswer(answer, (held) => ("export" in held ? held.export : undefined));
      if (finished || read.kind === "asking") return;
      finished = true;
      if (read.kind === "held") saveFile(`${filename}.html`, "text/html", read.value.html);
      setExporting(read.kind === "held" ? null : "refused");
      stopExport?.();
      stopExport = undefined;
    });
    if (finished) stop();
    else stopExport = stop;
  }

  const receipt = () => {
    const word = said();
    return word === null ? null : { text: say(lang(), word.key), alert: word.alert };
  };
  const heldVersion = () => {
    const value = version();
    return value === undefined ? null : short(value);
  };
  return (
    <HeadLookView
      place={{ folder: props.at.slice(0, cut()), name: name() }}
      version={heldVersion()}
      receipt={receipt()}
      wire={{ role: "status" }}
      actions={
        <>
          <Show when={state()?.file?.kind === "text"}>
            <Segmented
              label={say(lang(), "refrain_reading")}
              options={options()}
              held={props.reading}
              onPick={props.onPick}
            />
          </Show>
          <Show when={exporting() === "refused"}>
            <span class="text-note text-text-quiet" role="status">
              {say(lang(), "refrain_export_refused")}
            </span>
          </Show>
          <Show when={markdown()}>
            <Button
              label={say(lang(), "refrain_export_html")}
              tone="quiet"
              glyph="branch"
              {...(exporting() === "asking" ? { why: say(lang(), "refrain_export_asking") } : {})}
              onPress={exportHtml}
            />
          </Show>
          <Button
            label={say(lang(), "refrain_save")}
            tone="quiet"
            glyph="check"
            {...blocked()}
            onPress={() => props.session.save()}
          />
        </>
      }
    />
  );
}

export function HeadLookView(props: HeadLook) {
  return (
    <div class="head flex min-h-control flex-wrap items-center justify-between gap-x-base gap-y-tight border-b border-edge px-wide text-note whitespace-nowrap">
      <div class="where flex min-w-0 flex-[1_1_24ch] items-baseline gap-base">
        <p class="flex min-w-0 items-baseline truncate">
          <span class="truncate text-text-faint">{props.place.folder}</span>
          <span class="text-text-quiet">{props.place.name}</span>
        </p>
        <Show when={props.version}>
          {(hash) => <span class="shrink-0 text-text-faint">{hash()}</span>}
        </Show>
        <span
          {...props.wire}
          class={[
            "receipt inline-flex flex-none items-center gap-tight self-center text-text-quiet data-[alert=true]:text-alert data-[alert=true]:before:size-dot data-[alert=true]:before:rounded-pill data-[alert=true]:before:bg-alert",
            props.receipt === null ? "sr-only" : "",
          ]}
          data-alert={props.receipt?.alert ?? false}
        >
          {props.receipt?.text}
        </span>
      </div>
      <div class="flex shrink-0 items-center gap-snug">{props.actions}</div>
    </div>
  );
}
