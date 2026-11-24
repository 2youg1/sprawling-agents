// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, onCleanup, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { positionsOf } from "../../core/document_pos";
import { recorded, whole } from "../../core/document_windows";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { kib, moment } from "../../core/time";
import { ui } from "../../ui";
import type { Address, DocumentVersion } from "../../wire";
import Button from "../parts/button.solid";
import Empty from "../parts/empty.solid";
import EditorHost from "./editor_host";
import { Gathered } from "./gathered";
import { sideWire, type PairLook } from "./pair";
import Pair from "./pair.look.solid";
import { short } from "./reading";
import { saveFile } from "./saved_file";
import type { Session } from "./session";

interface VersionsProps {
  readonly at: Address;
  readonly session: Session;
  readonly label: string;
  readonly phrases: Readonly<Record<string, string>>;
}

const DRAFT = "draft";

export default function Versions(props: VersionsProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const state = storeValue(() => props.session.state);
  const asked = storeValue(() => u.conn.asking.ask({ versions: { at: props.at } }));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("versions" in answer ? answer.versions : undefined)),
  );
  const listed = createMemo(() => {
    const answer = read();
    if (answer.kind !== "held") return [];
    const found = new Set<string>();
    return answer.value.versions.filter((item) => {
      if (found.has(item.version)) return false;
      found.add(item.version);
      return true;
    });
  });

  function sourceOf(each: DocumentVersion): string {
    const source = each.source;
    if (source === "on_disk") return say(lang(), "refrain_version_on_disk");
    if (source === "shipped") return say(lang(), "refrain_version_shipped");
    if ("before" in source) return say(lang(), "refrain_version_outside");
    return `${say(lang(), "refrain_version_saved")} · ${moment(source.saved.at)}`;
  }

  function nameOf(each: DocumentVersion): string {
    const parts = [short(each.version), sourceOf(each)];
    if (each.bytes != null) parts.push(kib(each.bytes));
    if (!each.kept) parts.push(say(lang(), "refrain_version_not_kept"));
    return parts.join(" · ");
  }

  const draft = () => {
    const held = state();
    if (held === undefined || held.receipt.kind === "clean" || held.receipt.kind === "saved")
      return null;
    return held.editing?.text() ?? null;
  };
  const sides = createMemo(() => [
    ...(draft() === null
      ? []
      : [{ value: DRAFT, label: say(lang(), "refrain_version_draft"), kept: true }]),
    ...listed().map((each) => ({ value: each.version, label: nameOf(each), kept: each.kept })),
  ]);
  const choosable = createMemo(() => sides().filter((each) => each.kept));
  const [from, pickFrom] = createSignal<string | null>(null);
  const [to, pickTo] = createSignal<string | null>(null);
  const right = () => to() ?? choosable()[0]?.value ?? null;
  const left = () => from() ?? choosable().find((each) => each.value !== right())?.value ?? null;

  const pair = createMemo((): PairLook => {
    const answer = read();
    return {
      sides: [
        { key: "from", label: say(lang(), "refrain_from"), held: left() ?? "", pick: pickFrom },
        { key: "to", label: say(lang(), "refrain_to"), held: right() ?? "", pick: pickTo },
      ].map((side) => ({
        key: side.key,
        label: side.label,
        held: side.held,
        options: sides().map((each) => ({
          value: each.value,
          label: each.label,
          disabled: !each.kept,
        })),
        wire: sideWire(side.label, side.pick),
      })),
      more:
        answer.kind === "held" && answer.value.more ? say(lang(), "refrain_versions_more") : null,
    };
  });

  const format = createMemo(() => {
    const file = state()?.file;
    return file?.kind === "text" ? file.gathering.format : "plain";
  });
  const leftText = new Gathered(u.conn.asking);
  const rightText = new Gathered(u.conn.asking);
  const leftWindow = storeValue(leftText.state);
  const rightWindow = storeValue(rightText.state);
  onCleanup(() => {
    leftText.close();
    rightText.close();
  });

  function load(value: () => string | null, gathered: Gathered): void {
    const wanted = createMemo(() => {
      const version = value();
      if (state()?.versions.some((each) => each.version === version)) return null;
      return listed().find((each) => each.version === version)?.version ?? null;
    });
    createEffect(
      () => [wanted(), format()] as const,
      ([version, kind]) => {
        gathered.start(version === null ? null : recorded(version, kind));
      },
    );
  }
  load(left, leftText);
  load(right, rightText);

  function textOf(value: string | null, got: Gathered["value"]): string | null {
    if (value === null) return null;
    if (value === DRAFT) return draft();
    const held = state()?.versions.find((each) => each.version === value);
    if (held !== undefined) return held.text;
    if (got?.version !== value || !whole(got)) return null;
    return positionsOf(got.version, got.encoding, got.text).editor;
  }

  const a = createMemo(() => textOf(left(), leftWindow().value));
  const b = createMemo(() => textOf(right(), rightWindow().value));
  const lost = () => leftWindow().lost !== null || rightWindow().lost !== null;
  const nameOfSide = (value: string | null) =>
    sides().find((each) => each.value === value)?.label ?? "";
  const [host, setHost] = createSignal<HTMLDivElement>();
  createEffect(
    () => [host(), a(), b(), props.label, props.phrases, lost()] as const,
    ([parent, older, newer, label, phrases, unavailable]) => {
      if (
        parent === undefined ||
        older === null ||
        newer === null ||
        older === newer ||
        unavailable
      )
        return;
      let shown: { readonly destroy: () => void } | undefined;
      let gone = false;
      void import("./editing").then(({ openComparison }) => {
        if (!gone && parent.isConnected)
          shown = openComparison(parent, { from: older, to: newer }, label, phrases);
      });
      return () => {
        gone = true;
        shown?.destroy();
      };
    },
  );

  function exportComparison(): void {
    const older = a();
    const newer = b();
    if (older === null || newer === null) return;
    const l = lang();
    const label = props.label;
    const comparison = {
      name: label,
      from: { label: nameOfSide(left()), text: older },
      to: { label: nameOfSide(right()), text: newer },
      about: [say(l, "refrain_compared_source")],
    };
    void import("./formats/compared").then(({ comparedMarkdown, comparedName }) => {
      saveFile(comparedName(label), "text/markdown", comparedMarkdown(l, comparison));
    });
  }

  return (
    <Switch>
      <Match when={read().kind === "asking"}>
        <p class="p-pane text-note text-text-faint">{say(lang(), "refrain_versions_asking")}</p>
      </Match>
      <Match when={choosable().length < 2}>
        <Empty missing="refrain_versions_one" />
      </Match>
      <Match when={true}>
        <Pair {...pair()} />
        <Switch
          fallback={
            <>
              <div class="flex justify-end border-b border-edge px-wide py-tight">
                <Button
                  label={say(lang(), "refrain_export_comparison")}
                  tone="quiet"
                  onPress={exportComparison}
                />
              </div>
              <EditorHost
                ref={(node) => {
                  setHost(node);
                }}
              />
            </>
          }
        >
          <Match when={lost()}>
            <p class="p-pane text-note text-text-quiet">{say(lang(), "refrain_version_lost")}</p>
          </Match>
          <Match when={a() === null || b() === null}>
            <p class="p-pane text-note text-text-faint">{say(lang(), "refrain_version_reading")}</p>
          </Match>
          <Match when={a() === b()}>
            <p class="p-pane text-note text-text-quiet">
              {fill(say(lang(), "refrain_diff_same"), { version: nameOfSide(left()) })}
            </p>
          </Match>
        </Switch>
      </Match>
    </Switch>
  );
}
