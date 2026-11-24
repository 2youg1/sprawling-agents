// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo, createSignal } from "solid-js";
import { QUERIES } from "../../core/asking";
import { pursue } from "../../core/commands";
import { say } from "../../core/lang";
import { pursuitClause } from "../../core/pursuit";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
interface Props {
  readonly address: Address;
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function BuildingGoal(props: Props) {
  const u = ui();
  const lang = u.lang;
  const $lang = storeValue(lang);
  const city = u.conn.asking.ask(QUERIES.city);
  const $city = storeValue(city);
  const [goal, __rrSet0] = createSignal("");
  const pursuit = createMemo(() => {
    const held = $city();
    return held !== undefined && "city" in held
      ? held.city.pursuits.find((line) => line.addr === props.address)
      : undefined;
  });
  function setGoalNow(form: HTMLFormElement): void {
    const words = goal().trim();
    if (words === "") {
      form.querySelector("input")?.focus();
    } else if (u.send(pursue(props.address, { set: { goal: words } }))) {
      __rrSet0(() => "");
    }
  }
  return (
    <>
      <div
        class={"flex min-w-0 items-center gap-base"}
        role={"group"}
        aria-label={say($lang(), "bld_goal")}
      >
        <span class={"w-figure shrink-0 text-note text-text-faint"}>
          {say($lang(), "bld_goal")}
        </span>
        {(() => {
          const __rrBranch2 = createMemo(() => {
            const __rrCaptured1 = pursuit();
            return __rrCaptured1 === undefined
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
                      <span
                        class={[
                          "size-dot shrink-0 rounded-pill",
                          __rrActive3().__rrCaptured1.state === "running" ? "bg-accent" : "bg-mark",
                        ]}
                        aria-hidden={"true"}
                      />
                      <span class={"min-w-0 flex-1 truncate text-note text-text"}>
                        {__rrActive3().__rrCaptured1.goal}
                      </span>
                      <span class={"shrink-0 text-note text-text-faint"}>
                        {pursuitClause($lang(), __rrActive3().__rrCaptured1.verdict)}
                      </span>
                      <Button
                        label={
                          __rrActive3().__rrCaptured1.state === "running"
                            ? say($lang(), "bld_pause")
                            : say($lang(), "bld_resume")
                        }
                        tone={"quiet"}
                        onPress={() => {
                          u.send(
                            pursue(
                              props.address,
                              __rrActive3().__rrCaptured1.state === "running" ? "pause" : "resume",
                            ),
                          );
                        }}
                      />
                      <Button
                        label={say($lang(), "bld_clear")}
                        tone={"quiet"}
                        onPress={() => {
                          u.send(pursue(props.address, "clear"));
                        }}
                      />
                    </>
                  )}
                </Show>
              }
            >
              {(__rrActive3) => (
                <>
                  <form
                    class={"flex min-w-0 flex-1 items-center gap-base"}
                    onSubmit={(event) => {
                      event.preventDefault();
                      setGoalNow(event.currentTarget);
                    }}
                  >
                    <div class={"min-w-0 flex-1"}>
                      <Field
                        label={say($lang(), "bld_goal")}
                        labelling={"hidden"}
                        placeholder={say($lang(), "bld_goal_placeholder")}
                        value={goal()}
                        onInput={(value) => {
                          __rrSet0(() => value);
                        }}
                      />
                    </div>
                    <Button
                      label={say($lang(), "bld_pursue")}
                      type={"submit"}
                      tone={goal().trim() === "" ? "secondary" : "primary"}
                    />
                  </form>
                </>
              )}
            </Show>
          );
        })()}
      </div>
    </>
  );
}
