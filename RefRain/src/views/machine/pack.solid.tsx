// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { DoctorItem, DoctorNewest } from "../../wire";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import { offerOf, spelledOf, stateKey } from "../setup/dependencies";
import Versions from "./versions.solid";

export interface PackProps {
  readonly members: readonly DoctorItem[];
  readonly newest: Readonly<Record<string, DoctorNewest>>;
  readonly walking: boolean;
  readonly onInstall?: ((names: readonly string[]) => void) | undefined;
}

export default function Pack(props: PackProps) {
  const lang = storeValue(ui().lang);
  const pressable = createMemo(() =>
    props.members.filter((item) => offerOf(item) === "press").map((item) => item.name),
  );
  const wanting = () =>
    props.members.some((item) => !("present" in item.state) && item.need === "required");
  const complete = () =>
    props.members.every((item) => "present" in item.state || offerOf(item) === "by_hand");

  return (
    <li class="col-span-full flex min-w-0 flex-col gap-snug border-t border-edge pt-snug pb-base">
      <div class="flex min-w-0 flex-wrap items-center gap-snug">
        <span class="shrink-0 text-label font-label text-text">
          {say(lang(), "machine_pack_rust_tools")}
        </span>
        <Badge
          text={say(lang(), complete() ? "machine_present" : "machine_absent")}
          weight={wanting() ? "alert" : "quiet"}
          dot
        />
        <span class="min-w-0 flex-1" />
        <Show when={pressable().length > 0 && props.onInstall !== undefined}>
          <Button
            label={fill(say(lang(), "machine_pack_install"), { count: String(pressable().length) })}
            tone="secondary"
            loading={props.walking}
            onPress={() => props.onInstall?.(pressable())}
          />
        </Show>
      </div>
      <p class="text-note text-text-faint">{say(lang(), "machine_pack_rust_tools_note")}</p>
      <Show when={pressable().length > 0}>
        <p class="text-note text-text-faint">{say(lang(), "machine_pack_install_note")}</p>
      </Show>
      <ul class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-x-wide gap-y-snug">
        <For each={props.members} keyed={(item) => item.name}>
          {(member) => {
            const how = () => {
              const item = member();
              return !("present" in item.state) && offerOf(item) !== "press"
                ? spelledOf(item.install)
                : null;
            };
            return (
              <li class="flex min-w-0 flex-col gap-tight border-t border-edge pt-snug">
                <div class="flex min-w-0 items-center gap-snug">
                  <Show
                    when={member().homepage}
                    fallback={
                      <span class="shrink-0 font-mono text-label text-text">{member().name}</span>
                    }
                  >
                    {(href) => (
                      <a
                        class="shrink-0 font-mono text-label text-text underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
                        href={href()}
                        target="_blank"
                        rel="noreferrer"
                      >
                        {member().name}
                      </a>
                    )}
                  </Show>
                  <Badge
                    text={say(lang(), stateKey(member().state))}
                    weight={
                      "present" in member().state || member().need === "optional"
                        ? "quiet"
                        : "alert"
                    }
                    dot
                  />
                  <Show when={member().need === "optional"}>
                    <span class="shrink-0 text-note text-text-faint">
                      {say(lang(), "machine_optional")}
                    </span>
                  </Show>
                </div>
                <Versions item={member()} newest={props.newest[member().name]} />
                <Show when={how()}>
                  {(recipe) => (
                    <p class="min-w-0 break-words text-note text-text-faint">{recipe()}</p>
                  )}
                </Show>
              </li>
            );
          }}
        </For>
      </ul>
    </li>
  );
}
