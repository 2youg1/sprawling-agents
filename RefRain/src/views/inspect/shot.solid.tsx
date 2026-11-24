// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createEffect, createMemo, createSignal } from "solid-js";
import { onCleanup as onDestroy } from "solid-js";
import { storedObject } from "../../core/document_bytes";
import { fill, say } from "../../core/lang";
import { ui } from "../../ui";
import { StoredBytes } from "../editor/fetched";
import type { Picture } from "./reading";
interface Props {
  readonly picture: Picture;
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function InspectShot(props: Props) {
  const u = ui();
  const lang = u.lang;
  const $lang = storeValue(lang);
  const object = createMemo(() => storedObject(props.picture.image));
  const fetched = new StoredBytes(u.conn.asking);
  const fetchedWindow = fetched.state;
  const $fetchedWindow = storeValue(fetchedWindow);
  onDestroy(() => {
    fetched.close();
  });
  createEffect(
    () => [object()] as const,
    () => {
      const __rrStop4 = (() => {
        fetched.start(object());
      })();
      return typeof __rrStop4 === "function" ? __rrStop4 : undefined;
    },
  );
  const [source, __rrSet0] = createSignal<string | null>(null);
  createEffect(
    () =>
      [
        $fetchedWindow(),
        !($fetchedWindow().value?.kind !== "whole") ? props.picture : undefined,
      ] as const,
    () => {
      const __rrStop5 = (() => {
        const value = $fetchedWindow().value;
        if (value?.kind !== "whole") return;
        const url = URL.createObjectURL(
          new Blob([value.bytes], { type: props.picture.media_type }),
        );
        __rrSet0(() => url);
        return () => {
          URL.revokeObjectURL(url);
          __rrSet0(() => null);
        };
      })();
      return typeof __rrStop5 === "function" ? __rrStop5 : undefined;
    },
  );
  const said = createMemo(() =>
    object() === null ||
    $fetchedWindow().lost !== null ||
    $fetchedWindow().value?.kind === "too_large"
      ? "inspect_shot_lost"
      : "inspect_shot_reading",
  );
  return (
    <>
      <div class={"flex min-h-0 flex-1 flex-col gap-base bg-page px-wide py-pane"}>
        <p class={"shrink-0 text-note text-text-faint"}>
          {fill(say($lang(), "inspect_shot"), {
            width: String(props.picture.width),
            height: String(props.picture.height),
            media: props.picture.media_type,
          })}
        </p>
        <div class={"flex min-h-0 flex-1 items-center justify-center [container-type:size]"}>
          {(() => {
            const __rrBranch2 = createMemo(() => {
              const __rrCaptured1 = source();
              return __rrCaptured1 !== null
                ? { yes: { __rrCaptured1 }, no: undefined }
                : { yes: undefined, no: { __rrCaptured1 } };
            });
            return (
              <Show
                when={__rrBranch2().yes}
                fallback={
                  <Show when={__rrBranch2().no}>
                    {(__rrActive3) => (
                      <>
                        <div
                          class={
                            "flex items-center justify-center rounded-card border border-dashed border-edge-input"
                          }
                          style={{
                            "aspect-ratio":
                              String(props.picture.width) + " / " + String(props.picture.height),
                            width:
                              "min(100cqw, calc(100cqh * " +
                              String(props.picture.width) +
                              " / " +
                              String(props.picture.height) +
                              "))",
                          }}
                        >
                          <span class={"px-pane text-center text-note text-text-faint"}>
                            {say($lang(), said())}
                          </span>
                        </div>
                      </>
                    )}
                  </Show>
                }
              >
                {(__rrActive3) => (
                  <>
                    <img
                      class={
                        "block max-h-full max-w-full rounded-card border border-edge object-contain"
                      }
                      src={__rrActive3().__rrCaptured1}
                      width={props.picture.width}
                      height={props.picture.height}
                      alt={fill(say($lang(), "inspect_shot"), {
                        width: String(props.picture.width),
                        height: String(props.picture.height),
                        media: props.picture.media_type,
                      })}
                      style={{
                        "aspect-ratio":
                          String(props.picture.width) + " / " + String(props.picture.height),
                      }}
                    />
                  </>
                )}
              </Show>
            );
          })()}
        </div>
        <p class={"shrink-0 font-mono text-note break-all text-text-quiet select-all"}>
          {props.picture.image}
        </p>
      </div>
    </>
  );
}
