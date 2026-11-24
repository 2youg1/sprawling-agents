// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createEffect, createMemo, createSignal } from "solid-js";

import { release } from "../core/commands";
import { fill, say } from "../core/lang";
import { CITY, cityIsShut } from "../core/scope";
import { RELEASE_ALL } from "../core/slash";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import Banner from "./parts/banner.solid";
import Button from "./parts/button.solid";

export default function Banners() {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const link = storeValue(u.conn.state);
  const closing = storeValue(u.conn.closing);
  const unsent = storeValue(u.conn.unsent);
  const [lostAttempt, setLostAttempt] = createSignal<number | null>(null);

  createEffect(link, (next) => {
    if (next.kind === "backoff") setLostAttempt(next.attempt + 1);
    else if (next.kind === "live" || next.kind === "refused" || next.kind === "closed") {
      setLostAttempt(null);
    }
  });

  const closed = createMemo(() => link().kind === "closed");
  const halted = createMemo(() => cityIsShut(belief().halted));
  const closingWords = createMemo(() => {
    if (closing() !== "drain") return say(lang(), "close_interrupting");
    const count = belief().live.length;
    return count === 0
      ? say(lang(), "close_closing")
      : fill(say(lang(), "close_draining"), { n: String(count) });
  });

  const retryDetail = createMemo(() => {
    const attempt = lostAttempt();
    if (attempt === null) return undefined;
    const count = unsent();
    return [
      fill(say(lang(), "link_lost_detail"), { n: String(attempt) }),
      ...(count > 0 ? [fill(say(lang(), "link_unsent"), { n: String(count) })] : []),
    ].join(" · ");
  });

  const haltedDetail = createMemo(() => {
    const count = belief().cancelled;
    return count > 0 ? fill(say(lang(), "halt_frozen"), { n: String(count) }) : undefined;
  });

  return (
    <Show when={lostAttempt() !== null || halted() || closed() || closing() !== null}>
      <div class="col-[2/12] row-start-1 flex flex-col gap-snug pb-base narrow:col-span-full">
        <Show
          when={closed()}
          fallback={
            <Show when={closing() !== null}>
              <Banner text={closingWords()} weight="notice" />
            </Show>
          }
        >
          <Banner text={say(lang(), "close_done")} weight="alert" />
        </Show>
        <Show when={lostAttempt() !== null}>
          <Banner
            text={say(lang(), "link_lost")}
            detail={retryDetail()}
            weight="alert"
            action={
              <Button
                label={say(lang(), "link_retry_now")}
                tone="secondary"
                onPress={u.conn.retry}
              />
            }
          />
        </Show>
        <Show when={halted()}>
          <Banner
            text={say(lang(), "halt_title")}
            detail={haltedDetail()}
            weight="alert"
            action={
              <Button label={RELEASE_ALL} tone="secondary" onPress={() => u.send(release(CITY))} />
            }
          />
        </Show>
      </div>
    </Show>
  );
}
