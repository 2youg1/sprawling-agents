// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What came back from the far side, drawn beside the fields it is
// about. Two panels, because a probe and an attachment fail
// differently: an attachment that was turned away carries the city's
// own refusal, and a probe carries a staged reading of how far the
// call got.

import { For, Show } from "solid-js";

import { stoppedAt } from "../../../core/probed";
import type { Probed } from "../../../core/probed";
import type { Key } from "../../../core/lang";
import type { AxCode, AxError } from "../../../wire";
import { useSay } from "../../../ui";
import { Tip } from "../../parts/tip";

// What a person can do about a refusal, by the code it carries. The
// city's own `recovery` speaks to the runtime that must retry or fail
// over, so it is copied rather than shown; this table is the half
// written for the person filling in the form.
const NEXT_STEP: readonly (readonly [AxCode, Key])[] = [
  ["E_PROVIDER", "setup_next_provider"],
  ["E_TIMEOUT", "setup_next_timeout"],
  ["E_CREDENTIAL_MISSING", "setup_next_credential"],
  ["E_CONFIG_INVALID", "setup_next_config"],
  ["E_INVALID_ARGS", "setup_next_config"],
  ["E_ENDPOINT_DIALECT_UNSUPPORTED", "setup_next_dialect"],
  ["E_WIRE_MISMATCH", "setup_next_dialect"],
];

function nextStep(code: AxCode): Key {
  return NEXT_STEP.find(([known]) => known === code)?.[1] ?? "setup_next_other";
}

// A refusal the city sent back, which is what an attachment that failed
// produces. A probe answers with a reading instead, and that is the
// other component below.
export function Refusal(props: { readonly error: AxError; readonly host: string }) {
  const say = useSay();
  return (
    <div role="alert" class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note">
      <p class="font-label text-alert">{say("setup_probe_failed", { host: props.host })}</p>
      <p class="mt-tight text-text-quiet">{say(nextStep(props.error.code))}</p>
      <div class="mt-snug flex items-center gap-snug text-text-faint">
        <span class="font-mono">{props.error.code}</span>
        <button
          type="button"
          class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-raised"
          onClick={() => {
            void navigator.clipboard.writeText(
              `${props.error.code}\n${props.error.action}\n${props.error.subject}\n${props.error.recovery}`,
            );
          }}
        >
          {say("setup_copy_details")}
        </button>
      </div>
    </div>
  );
}

// Where the call stopped, stage by stage, as the city measured it.
//
// Four stages, each with the word the city wrote for it, and above them
// the one sentence a person can act on. The stages are shown even when
// the call went through: a 401 from a host that resolved and answered in
// 90 ms is a key, and somebody sent to look at a proxy instead has been
// sent the wrong way.
export function Reachability(props: { readonly probed: Probed }) {
  const say = useSay();
  const reach = () => props.probed.reach;
  const failed = () => props.probed.failure;
  return (
    <Show when={reach()}>
      {(found) => (
        <div
          role="status"
          class={`rounded-card border bg-chrome px-base py-snug text-note ${failed() === null ? "border-edge-panel" : "border-alert/50"}`}
        >
          <p class={`font-label ${failed() === null ? "text-text" : "text-alert"}`}>
            {say("setup_probe_read", { host: found().host })}
          </p>
          <p class="mt-tight text-text-quiet">{say(stoppedAt(found()))}</p>
          <dl class="mt-snug grid grid-cols-2 gap-x-base gap-y-tight text-text-faint">
            <For
              each={[
                ["setup_reach_dns", found().named],
                ["setup_reach_tcp", found().connected],
                ["setup_reach_http", found().answered],
                ["setup_reach_proxy", found().through],
              ] as const}
            >
              {([label, stage]) => (
                <>
                  <dt>{say(label)}</dt>
                  {/* wording-ok: the word the city itself wrote for this stage */}
                  <dd class="min-w-0 font-mono">
                    <Tip text={stage.detail ?? stage.state}>
                      {(hint) => (
                        <span class="wrap-anywhere" tabindex={0} aria-describedby={hint}>
                          {stage.figure === null ? stage.state : `${stage.state} ${String(stage.figure)}`}
                        </span>
                      )}
                    </Tip>
                  </dd>
                </>
              )}
            </For>
          </dl>
          <div class="mt-snug flex items-center gap-snug text-text-faint">
            <span>{say("setup_reach_elapsed", { ms: String(found().elapsedMs) })}</span>
            <Show when={failed()}>
              {(why) => (
                <>
                  <span class="font-mono">{why().code}</span>
                  <button
                    type="button"
                    class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-raised"
                    onClick={() => {
                      void navigator.clipboard.writeText(`${why().code} ${why().subject}`);
                    }}
                  >
                    {say("setup_copy_details")}
                  </button>
                </>
              )}
            </Show>
          </div>
        </div>
      )}
    </Show>
  );
}
