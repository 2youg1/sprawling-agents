// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this building can do: the city's library and the building's own
// shelf, in one list with the shelf named on each row. A row says
// whether the reading room admits it and how many runs were frozen with
// it; opening one shows the document itself, through the same file view
// the tree opens a file with.

import { For, Show, createMemo } from "solid-js";

import type { Address, SkillLine, SkillsAnswer } from "../../wire";
import { useSay, useUi } from "../../ui";
import type { Picked } from "./tree";

function Row(props: { readonly skill: SkillLine; readonly onOpen: () => void }) {
  const say = useSay();
  return (
    <li class="border-b border-g1">
      <button
        type="button"
        class="flex w-full min-w-0 items-baseline gap-base py-snug text-left text-note hover:text-text"
        onClick={() => {
          props.onOpen();
        }}
      >
        <span class="shrink-0 font-mono text-text-quiet">{props.skill.name}</span>
        <span class="shrink-0 text-text-disabled">
          {props.skill.shelf === "building" ? say("skills_shelf") : say("skills_library")}
        </span>
        <span class="min-w-0 flex-1 truncate text-text-faint">{props.skill.disclosure}</span>
        <span class="shrink-0 text-text-disabled">
          {props.skill.admitted ? say("skills_admitted") : say("skills_not_admitted")}
        </span>
        <span class="shrink-0 text-text-disabled">
          {props.skill.pinned_by.length > 0
            ? say("skills_used_by", { n: String(props.skill.pinned_by.length) })
            : say("skills_used_never")}
        </span>
      </button>
    </li>
  );
}

export function Skills(props: { readonly building: Address; readonly onPick: (picked: Picked) => void }) {
  const ui = useUi();
  const say = useSay();
  const asked = createMemo(() => ui.conn.asking.ask({ skills: { building: props.building } }));
  const answer = createMemo<SkillsAnswer | undefined>(() => {
    const held = asked()();
    return held !== undefined && "skills" in held ? held.skills : undefined;
  });
  return (
    <div>
      <h2 class="mb-base text-heading font-heading">{say("bld_skills")}</h2>
      <Show when={answer()} fallback={<p class="text-text-disabled">…</p>}>
        {(held) => (
          <>
            <Show
              when={held().skills.length > 0}
              fallback={<p class="text-text-faint">{say("skills_empty")}</p>}
            >
              <ul>
                <For each={held().skills}>
                  {(skill) => (
                    <Row
                      skill={skill}
                      onOpen={() => {
                        props.onPick({ at: skill.at, kind: "file" });
                      }}
                    />
                  )}
                </For>
              </ul>
            </Show>
            <Show when={held().missing.length > 0}>
              <p class="mt-base text-note text-alert">
                {say("skills_missing")}: {held().missing.join(" · ")}
              </p>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}
