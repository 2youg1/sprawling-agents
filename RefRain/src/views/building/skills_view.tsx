// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { fill, say, type Key, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, Query, RunId, SkillLine, SkillShelf } from "../../wire";
import Unanswered from "../parts/unanswered.solid";
import SkillRow from "./skill_row.look.solid";
import type { Picked } from "./tree.solid";

interface Props {
  readonly building: Address;
  readonly onPick: (picked: Picked) => void;
  readonly newest?: RunId | undefined;
}

function shelfName(shelf: SkillShelf, lang: Lang): string {
  if (shelf === "library") return say(lang, "skills_library");
  if ("building" in shelf) return say(lang, "skills_shelf");
  return fill(say(lang, "skills_external"), {
    n: String(shelf.external.index),
    path: shelf.external.path,
  });
}

function skillId(skill: SkillLine): string {
  const shelf = skill.shelf;
  if (shelf === "library") return `library·${skill.name}`;
  if ("building" in shelf) return `${shelf.building}·${skill.name}`;
  return `external:${String(shelf.external.index)}·${skill.name}`;
}

export default function Skills(props: Props) {
  const u = ui();
  const lang = storeValue(u.lang);
  const question = createMemo<Query>(() => ({ skills: { building: props.building } }));
  const asked = storeValue(() => u.conn.asking.ask(question()));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("skills" in answer ? answer.skills : undefined)),
  );
  const held = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value : undefined;
  };
  const missing = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const word = (key: Key) => say(lang(), key);

  function changed(skill: SkillLine): string | undefined {
    return props.newest === undefined || !skill.admitted || skill.pinned_by.includes(props.newest)
      ? undefined
      : word("skills_changed");
  }

  return (
    <div>
      <h3 class="mb-base text-note text-text-faint">{word("bld_skills")}</h3>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={missing()}>
          {(unavailable) => (
            <Unanswered
              query={unavailable().query}
              asked={question()}
              reason={unavailable().reason}
            />
          )}
        </Match>
        <Match when={held()}>
          {(answer) => (
            <>
              <Show
                when={answer().skills.length > 0}
                fallback={<p class="text-text-faint">{word("skills_empty_next")}</p>}
              >
                <ul>
                  <For each={answer().skills} keyed={skillId}>
                    {(skill) => (
                      <li class="border-b border-edge">
                        <SkillRow
                          name={skill().name}
                          summary={skill().disclosure}
                          shelf={shelfName(skill().shelf, lang())}
                          at=""
                          admitted={word(
                            skill().admitted ? "skills_admitted" : "skills_not_admitted",
                          )}
                          used={
                            skill().pinned_by.length > 0
                              ? fill(word("skills_used_by"), {
                                  n: String(skill().pinned_by.length),
                                })
                              : word("skills_used_never")
                          }
                          changed={changed(skill())}
                          wire={{ type: "button", disabled: true, onclick: () => undefined }}
                        />
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
              <Show when={answer().missing.length > 0}>
                <p class="mt-base text-note text-alert">
                  {word("skills_missing")}: {answer().missing.join(" · ")}
                </p>
              </Show>
            </>
          )}
        </Match>
      </Switch>
    </div>
  );
}
