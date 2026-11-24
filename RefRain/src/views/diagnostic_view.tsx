// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result } from "effect";
import { createMemo, createSignal, Show } from "solid-js";
import { diagnosticToml } from "../core/diagnostic";
import { say } from "../core/lang";
import type { Lang } from "../core/lang";
import type { AxError } from "../wire";

export interface DiagnosticProps {
  readonly error: AxError;
  readonly lang: Lang;
  readonly onDraft?: ((text: string) => void) | undefined;
}

export function DiagnosticView(props: DiagnosticProps) {
  const diagnostic = createMemo(() => diagnosticToml(props.error));
  const [status, setStatus] = createSignal<"copied" | "blocked" | null>(null);
  function copy(): void {
    const copied = Result.try(() => navigator.clipboard.writeText(diagnostic()));
    if (Result.isFailure(copied)) {
      setStatus("blocked");
      return;
    }
    void copied.success.then(
      () => {
        setStatus("copied");
      },
      () => {
        setStatus("blocked");
      },
    );
  }
  return (
    <details class="text-note">
      <summary>{say(props.lang, "diagnostic_details")}</summary>
      <pre class="max-h-output overflow-auto whitespace-pre-wrap wrap-anywhere">{diagnostic()}</pre>
      <button data-control="surface" type="button" class="text-text underline" onClick={copy}>
        {say(props.lang, "diagnostic_copy")}
      </button>
      <Show when={props.onDraft}>
        <button
          data-control="surface"
          type="button"
          class="ms-snug text-text underline"
          onClick={() => props.onDraft?.(diagnostic())}
        >
          {say(props.lang, "diagnostic_to_draft")}
        </button>
      </Show>
      <Show when={status()}>
        {(result) => (
          <p role="status">
            {say(
              props.lang,
              result() === "copied" ? "diagnostic_copied" : "diagnostic_copy_blocked",
            )}
          </p>
        )}
      </Show>
    </details>
  );
}
