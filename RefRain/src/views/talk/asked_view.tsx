// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Match, Show, Switch } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { ContentAnswer, Locator } from "../../wire";
import { shownOf } from "./asked";

export default function Asked(props: {
  readonly locator: Locator;
  readonly content?: ContentAnswer | undefined;
}) {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(() => u.conn.asking.ask({ content: { locator: props.locator } }));
  const shown = createMemo(() => {
    const answer = asked();
    return shownOf(
      lang(),
      props.content ?? (answer !== undefined && "content" in answer ? answer.content : undefined),
    );
  });
  return (
    <div class="my-snug">
      <details class="text-note" open>
        <summary class="cursor-pointer text-text-faint transition-colors ease-leave hover:text-text-quiet hover:ease-arrive">
          {say(lang(), "wait_asks")}
        </summary>
        <Switch fallback={<p class="text-text-faint">…</p>}>
          <Match when={shown().kind === "binary" ? shown() : undefined}>
            {(held) => (
              <p class="text-text-faint">
                {(() => {
                  const value = held();
                  return value.kind === "binary" ? value.said : "";
                })()}
              </p>
            )}
          </Match>
          <Match when={shown().kind === "text" ? shown() : undefined}>
            {(held) => {
              const read = () => {
                const value = held();
                return "text" in value ? value : null;
              };
              return (
                <>
                  <pre class="mt-tight max-h-output overflow-auto rounded-card border border-edge bg-page p-snug font-mono text-note whitespace-pre-wrap text-text-quiet">
                    {read()?.text}
                  </pre>
                  <Show when={read()?.cut}>{(why) => <p class="text-text-faint">{why()}</p>}</Show>
                </>
              );
            }}
          </Match>
        </Switch>
      </details>
    </div>
  );
}
