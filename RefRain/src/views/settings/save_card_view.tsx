// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, Show } from "solid-js";

import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import Glyph from "../parts/glyph.solid";
import { pressable, standingOf } from "./card";
import type { Saving } from "./saving";

interface SaveCardProps {
  readonly title: string;
  readonly note: string;
  readonly settled: string;
  readonly saveLabel?: string;
  readonly saving: Saving;
  readonly onSave: () => void;
  readonly onCheck?: () => void;
  readonly blocked?: Key | undefined;
  readonly children: JSX.Element;
}

export default function SaveCard(props: SaveCardProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const standing = () => standingOf(props.saving, props.settled, true, word);
  const error = () => {
    const saving = props.saving;
    return saving.kind === "refused" ? saving.error : undefined;
  };
  const check = () => props.saving.kind === "unverified" && props.onCheck !== undefined;
  const why = () =>
    props.blocked === undefined
      ? pressable(props.saving) || props.saving.kind === "saving"
        ? undefined
        : word("saving_nothing")
      : word(props.blocked);

  const button = createMemo(() => {
    const stop = why();
    return stop === undefined ? {} : { why: stop };
  });

  return (
    <div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
      <div class="flex flex-col gap-hair">
        <span class="text-label font-label text-text">{props.title}</span>
        <p class="text-note text-text-faint">{props.note}</p>
      </div>
      {props.children}
      <div class="mt-tight flex items-center gap-base">
        <span class="flex min-w-0 flex-1 flex-col gap-hair text-note" role="status">
          <Show when={standing().word !== null}>
            <span
              class={[
                "inline-flex items-center gap-tight",
                standing().weight === "alert" ? "text-alert" : "text-text-quiet",
              ]}
            >
              <Show when={standing().saved}>
                <Glyph name="check" size="sm" class="shrink-0" />
              </Show>
              {standing().word}
            </span>
          </Show>
          <Show when={standing().detail !== null}>
            <span class="text-text-faint">{standing().detail}</span>
          </Show>
        </span>
        <Button
          label={check() ? word("saving_check") : (props.saveLabel ?? word("saving_save"))}
          tone="primary"
          glyph={check() ? "search" : undefined}
          result={
            props.saving.kind === "saved"
              ? "success"
              : props.saving.kind === "refused"
                ? "refused"
                : props.saving.kind === "unverified"
                  ? "unknown"
                  : undefined
          }
          loading={props.saving.kind === "saving"}
          {...button()}
          onPress={() => {
            if (check()) props.onCheck?.();
            else props.onSave();
          }}
        />
      </div>
      <Show when={error()}>
        {(diagnostic) => <DiagnosticView error={diagnostic()} lang={lang()} />}
      </Show>
    </div>
  );
}
