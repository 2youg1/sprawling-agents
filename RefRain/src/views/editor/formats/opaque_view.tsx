// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, Match, onCleanup, Show, Switch } from "solid-js";
import { readAnswer } from "../../../core/answered";
import { DRAWN_BYTES_MAX } from "../../../core/document_bytes";
import { fill, say } from "../../../core/lang";
import { storeValue } from "../../../core/solid_store";
import { kib } from "../../../core/time";
import { ui } from "../../../ui";
import type { Address, B3Hash } from "../../../wire";
import { StoredBytes } from "../fetched";
import Against from "./against.look.solid";
import { againstLookOf } from "./against";
import Compare from "./compare_view";
import Docx from "./docx_view";
import { drawnAs, NAME, TOOL } from "./format";
import Label from "./label.solid";
import { PdfLoading } from "./pdf_loading";

export default function Opaque(props: {
  readonly path: Address;
  readonly version: B3Hash;
  readonly bytes: number;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const format = createMemo(() => {
    const found = drawnAs(props.path);
    return found === "pdf" || found === "docx" ? found : undefined;
  });
  const name = () => props.path.slice(props.path.lastIndexOf("/") + 1);
  const drawable = () => format() !== undefined && props.bytes <= DRAWN_BYTES_MAX;
  const current = new StoredBytes(u.conn.asking);
  const other = new StoredBytes(u.conn.asking);
  onCleanup(() => {
    current.close();
    other.close();
  });
  const mine = storeValue(current.state);
  const theirs = storeValue(other.state);
  const [against, setAgainst] = createSignal<B3Hash | null>(null);
  createEffect(
    () => (drawable() ? props.version : null),
    (version) => current.start(version),
  );
  createEffect(
    () => (drawable() ? against() : null),
    (version) => other.start(version),
  );
  const asked = storeValue(() => u.conn.asking.ask({ versions: { at: props.path } }));
  const others = createMemo(() => {
    const listed = readAnswer(asked(), (held) => ("versions" in held ? held.versions : undefined));
    if (listed.kind !== "held") return [];
    return listed.value.versions.filter(
      (each, index, all) =>
        each.kept &&
        each.version !== props.version &&
        all.findIndex((seen) => seen.version === each.version) === index,
    );
  });
  const drawn = createMemo(() => {
    const value = mine().value;
    return value?.kind === "whole" ? value.bytes : undefined;
  });
  const progress = createMemo(() => {
    const value = mine().value;
    return value?.kind === "fetching" ? value.through : 0;
  });
  const compared = createMemo(() => {
    const from = theirs().value;
    const to = drawn();
    const version = against();
    return from?.kind === "whole" && to !== undefined && version !== null
      ? { from: { version, bytes: from.bytes }, to: { version: props.version, bytes: to } }
      : undefined;
  });
  return (
    <Show
      when={format()}
      fallback={
        <p class="px-wide py-snug text-note text-text-quiet">
          {fill(say(lang(), "file_binary"), { kib: kib(props.bytes) })}
        </p>
      }
    >
      {(kind) => (
        <Show
          when={drawable()}
          fallback={
            <>
              <Label format={kind()} tool={TOOL[kind()]} settings={[]} version={props.version} />
              <p class="px-wide py-snug text-note text-text-quiet">
                {fill(say(lang(), "format_too_large"), {
                  format: NAME[kind()],
                  kib: kib(props.bytes),
                  most: kib(DRAWN_BYTES_MAX),
                })}
              </p>
            </>
          }
        >
          <Show when={others().length > 0}>
            <Against {...againstLookOf(others(), against(), lang(), setAgainst)} />
          </Show>
          <Switch
            fallback={
              <>
                <Label format={kind()} tool={TOOL[kind()]} settings={[]} version={props.version} />
                <p class="px-wide py-snug text-note text-text-faint">
                  {fill(say(lang(), "format_fetching"), {
                    format: NAME[kind()],
                    through: kib(progress()),
                    kib: kib(props.bytes),
                  })}
                </p>
              </>
            }
          >
            <Match when={mine().lost !== null || theirs().lost !== null}>
              <p class="px-wide py-snug text-note text-text-quiet">{say(lang(), "format_lost")}</p>
            </Match>
            <Match when={compared()}>
              {(both) => (
                <Compare format={kind()} from={both().from} to={both().to} name={name()} />
              )}
            </Match>
            <Match when={against() === null ? drawn() : undefined}>
              {(bytes) => (
                <Show
                  when={kind() === "pdf"}
                  fallback={<Docx bytes={bytes()} version={props.version} name={name()} />}
                >
                  <PdfLoading bytes={bytes()} version={props.version} name={name()} lang={lang()} />
                </Show>
              )}
            </Match>
          </Switch>
        </Show>
      )}
    </Show>
  );
}
