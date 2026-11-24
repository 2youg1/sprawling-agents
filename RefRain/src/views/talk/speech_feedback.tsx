// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch } from "solid-js";
import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { DiagnosticView } from "../diagnostic_view";
import Button from "../parts/button.solid";
import { failedLook } from "./refused";
import type { SpeechResult, SpeechState } from "./speech";

const failureWords: Readonly<Partial<Record<SpeechResult, Key>>> = {
  "image-unavailable": "talk_attachment_image_unavailable",
  "image-failed": "talk_attachment_image_failed",
  offline: "talk_attachment_offline",
};

export interface SpeechFeedbackProps {
  readonly state: SpeechState;
  readonly problem: SpeechResult | null;
  readonly lang: Lang;
  readonly recheck: () => void;
  readonly onDraft?: ((text: string) => void) | undefined;
}

export function SpeechFeedback(props: SpeechFeedbackProps) {
  const refusal = () => {
    const state = props.state;
    if (state.kind !== "refused") return;
    const error = state.error;
    return {
      error,
      card: failedLook(props.lang, say(props.lang, "talk_failed_refused"), error, {
        settings: "decided",
        onRetry: undefined,
      }),
    };
  };
  const problem = () => (props.problem === null ? undefined : failureWords[props.problem]);
  return (
    <Switch>
      <Match when={refusal()}>
        {(notice) => (
          <div
            class="mb-snug flex max-h-composer flex-col gap-tight overflow-y-auto rounded-panel border border-edge-panel p-snug"
            role="alert"
          >
            <p class="text-text">{notice().card.head}</p>
            <For each={notice().card.lines}>
              {(line) => <p class="text-text-quiet">{line.text}</p>}
            </For>
            <For each={notice().card.ways}>
              {(way) => (
                <Show when={way.kind === "link" ? way : undefined}>
                  {(link) => (
                    <a class="text-text underline" href={link().href}>
                      {link().text}
                    </a>
                  )}
                </Show>
              )}
            </For>
            <DiagnosticView error={notice().error} lang={props.lang} onDraft={props.onDraft} />
          </div>
        )}
      </Match>
      <Match when={props.state.kind === "pending" || props.state.kind === "preparing"}>
        <p role="status">{say(props.lang, "talk_delivery_pending")}</p>
      </Match>
      <Match when={props.state.kind === "unknown"}>
        <p role="status">{say(props.lang, "talk_delivery_unknown")}</p>
        <Button
          label={say(props.lang, "talk_recheck_receipt")}
          tone="quiet"
          glyph="search"
          onPress={props.recheck}
        />
      </Match>
      <Match when={problem()}>{(key) => <p role="status">{say(props.lang, key())}</p>}</Match>
    </Switch>
  );
}
