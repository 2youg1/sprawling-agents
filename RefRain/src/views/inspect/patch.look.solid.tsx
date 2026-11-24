// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Accessor } from "solid-js";
import { For, Match, Show, Switch } from "solid-js";

import Inked from "../parts/inked.solid";
import type { PatchLook, PatchRow } from "./patch";
import "./patch.solid.css";

type Code = Extract<PatchRow, { readonly kind: "code" }>;
type Tone = Code["tone"];

const WASH: Record<Tone, string> = {
  added: "bg-accent/12",
  removed: "bg-alert/12",
  context: "",
};
const MARK: Record<Tone, string> = {
  added: "text-accent",
  removed: "text-alert",
  context: "text-text-faint",
};

function CodeRow(props: { readonly row: Code; readonly source: string }) {
  return (
    <div
      class={["code grid pr-wide", WASH[props.row.tone], props.row.chosen && "chosen"]}
      data-line={props.row.key}
    >
      <Show
        when={props.row.quote}
        fallback={
          <span class={["pr-pane text-right select-none", MARK[props.row.tone]]}>
            {props.row.place}
          </span>
        }
      >
        {(quote) => (
          <a
            href={quote().href}
            aria-label={quote()["aria-label"]}
            onClick={() => quote().onclick()}
            class={["number pr-pane text-right select-none hover:text-text", MARK[props.row.tone]]}
          >
            {props.row.place}
          </a>
        )}
      </Show>
      <span class={["select-none", MARK[props.row.tone]]} aria-hidden="true">
        {props.row.sign}
      </span>
      <span class="whitespace-pre text-text">
        <Inked text={props.row.text} source={props.source} />
      </span>
    </div>
  );
}

function PatchLine(props: { readonly row: Accessor<PatchRow>; readonly source: string }) {
  const hunk = () => {
    const row = props.row();
    return row.kind === "hunk" ? row : undefined;
  };
  const code = () => {
    const row = props.row();
    return row.kind === "code" ? row : undefined;
  };
  return (
    <Switch
      fallback={
        <div class={["lead text-text-faint", props.row().kind === "head" && "whitespace-pre"]}>
          {props.row().text}
        </div>
      }
    >
      <Match when={hunk()}>
        {(row) => (
          <div class="lead my-tight flex gap-base bg-raised pr-wide text-text-faint">
            <Show when={row().folded !== undefined}>
              <span class="shrink-0">{row().folded}</span>
            </Show>
            <span class="whitespace-pre">{row().text}</span>
          </div>
        )}
      </Match>
      <Match when={code()}>{(row) => <CodeRow row={row()} source={props.source} />}</Match>
    </Switch>
  );
}

export default function PatchLookView(look: PatchLook) {
  return (
    <div class="patch inspect-patch w-max min-w-full py-snug font-mono text-note">
      <For each={look.rows} keyed={(row) => row.key}>
        {(row) => <PatchLine row={row} source={look.source} />}
      </For>
    </div>
  );
}
