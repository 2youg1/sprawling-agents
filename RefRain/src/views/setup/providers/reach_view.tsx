// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createUniqueId, For, Show } from "solid-js";

import { fill, say, type Key } from "../../../core/lang";
import { refusalOf, stoppedAt, type Probed } from "../../../core/probed";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { AxCode, AxError } from "../../../wire";
import { DiagnosticView } from "../../diagnostic_view";
import Button from "../../parts/button.solid";
import ControlTip from "../../parts/control_tip";

interface ReachProps {
  readonly probed: Probed | null;
  readonly error: AxError | null;
  readonly host: string;
}

const NEXT_STEP: Partial<Record<AxCode, Key>> = {
  E_PROVIDER: "setup_next_provider",
  E_TIMEOUT: "setup_next_timeout",
  E_CREDENTIAL_MISSING: "setup_next_credential",
  E_CONFIG_INVALID: "setup_next_config",
  E_INVALID_ARGS: "setup_next_config",
  E_ENDPOINT_DIALECT_UNSUPPORTED: "setup_next_dialect",
  E_WIRE_MISMATCH: "setup_next_dialect",
};

export default function Reach(props: ReachProps) {
  const lang = storeValue(ui().lang);
  const word = (key: Key) => say(lang(), key);
  const reach = () => props.probed?.reach ?? undefined;
  const failed = () => props.probed?.failure ?? null;
  const stages = createMemo(() => {
    const actual = reach();
    return actual === undefined
      ? []
      : ([
          ["setup_reach_dns", actual.named],
          ["setup_reach_tcp", actual.connected],
          ["setup_reach_http", actual.answered],
          ["setup_reach_proxy", actual.through],
        ] as const);
  });

  return (
    <>
      <Show when={props.error}>
        {(error) => (
          <div
            role="alert"
            class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note"
          >
            <p class="font-label text-alert">
              {refusalOf(error().code) === "reach"
                ? fill(word("setup_probe_failed"), { host: props.host })
                : word("setup_form_refused")}
            </p>
            <p class="mt-tight text-text-quiet">
              {word(NEXT_STEP[error().code] ?? "setup_next_other")}
            </p>
            <DiagnosticView error={error()} lang={lang()} />
          </div>
        )}
      </Show>
      <Show when={reach()}>
        {(actual) => (
          <div
            role="status"
            class={[
              "rounded-card border bg-chrome px-base py-snug text-note",
              failed() === null ? "border-edge-panel" : "border-alert/50",
            ]}
          >
            <p class={["font-label", failed() === null ? "text-text" : "text-alert"]}>
              {fill(word("setup_probe_read"), { host: actual().host })}
            </p>
            <p class="mt-tight text-text-quiet">{word(stoppedAt(actual(), failed()))}</p>
            <dl class="mt-snug grid grid-cols-2 gap-x-base gap-y-tight text-text-faint">
              <For each={stages()} keyed={(stage) => stage[0]}>
                {(stage) => {
                  const id = createUniqueId();
                  return (
                    <>
                      <dt>{word(stage()[0])}</dt>
                      <dd class="min-w-0 font-mono">
                        <ControlTip id={id} text={stage()[1].detail ?? stage()[1].state}>
                          <span class="wrap-anywhere" tabindex="0" aria-describedby={id}>
                            {stage()[1].figure === null
                              ? stage()[1].state
                              : `${stage()[1].state} ${String(stage()[1].figure)}`}
                          </span>
                        </ControlTip>
                      </dd>
                    </>
                  );
                }}
              </For>
            </dl>
            <div class="mt-snug flex items-center gap-snug text-text-faint">
              <span>{fill(word("setup_reach_elapsed"), { ms: String(actual().elapsedMs) })}</span>
              <Show when={failed()}>
                {(error) => (
                  <>
                    <span class="font-mono">{error().code}</span>
                    <Button
                      label={word("setup_copy_details")}
                      glyph="copy"
                      tone="quiet"
                      onPress={() => {
                        void navigator.clipboard.writeText(`${error().code} ${error().subject}`);
                      }}
                    />
                  </>
                )}
              </Show>
            </div>
          </div>
        )}
      </Show>
    </>
  );
}
