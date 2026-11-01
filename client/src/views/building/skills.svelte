<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What this building can do: the city's library and the building's
  // own shelf, in one list with the shelf named on each row. A row says
  // whether the reading room admits it, how many runs were frozen with
  // it, and whether the building's newest run was frozen with another
  // version of it. City and building shelves are CAS indexes, and an
  // external shelf is outside the city: none is a document address to
  // hand `Query::Document`, so the row names its source without a file link.
  import type { Lang } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import type { Address, SkillLine, SkillShelf } from "../../wire";

  interface Place {
    // Already in the person's language.
    readonly word: string;
    // Absent for every CAS-indexed shelf, which is not a document address.
    readonly at: Address | null;
  }

  function placeOf(shelf: SkillShelf, lang: Lang): Place {
    if (shelf === "library") return { word: say(lang, "skills_library"), at: null };
    if ("building" in shelf) return { word: say(lang, "skills_shelf"), at: null };
    return {
      word: fill(say(lang, "skills_external"), {
        n: String(shelf.external.index),
        path: shelf.external.path,
      }),
      at: null,
    };
  }

  // A row's identity: the same name may sit on two shelves, so the name
  // alone is not the row.
  function skillId(skill: SkillLine): string {
    const shelf = skill.shelf;
    if (shelf === "library") return `library·${skill.name}`;
    if ("building" in shelf) return `${shelf.building}·${skill.name}`;
    return `external:${String(shelf.external.index)}·${skill.name}`;
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import type { Query, RunId } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import SkillRow from "./skill_row.look.svelte";
  import type { Picked } from "./tree.svelte";

  interface Props {
    readonly building: Address;
    readonly onPick: (picked: Picked) => void;
    // The building's newest run whose prompt was frozen, if any: an
    // admitted skill that run did not pin at today's version is marked.
    readonly newest?: RunId | undefined;
  }

  const { building, onPick, newest }: Props = $props();

  // Whether the next run reads another version of this skill than the
  // newest run was frozen with: admitted, and not pinned by that run at
  // the hash the shelf holds now.
  function changed(skill: SkillLine): string | undefined {
    if (newest === undefined || !skill.admitted || skill.pinned_by.includes(newest))
      return undefined;
    return say($lang, "skills_changed");
  }

  const u = ui();
  const lang = u.lang;

  const question = $derived<Query>({ skills: { building } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("skills" in held ? held.skills : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);

  // A row this city cannot open is not a button's promise. An external
  // shelf is read-only and lives outside every address this client can
  // ask about, so the row states where it came from and stops there.
  function open(at: Address | null): void {
    if (at !== null) {
      onPick({ at, kind: "file" });
    }
  }
</script>

<div>
  <h3 class="mb-base text-note text-text-faint">{say($lang, "bld_skills")}</h3>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if answer === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    {#if answer.skills.length > 0}
      <ul>
        {#each answer.skills as skill (skillId(skill))}
          {@const place = placeOf(skill.shelf, $lang)}
          <li class="border-b border-edge">
            <SkillRow
              name={skill.name}
              summary={skill.disclosure}
              shelf={place.word}
              at={place.at ?? ""}
              admitted={skill.admitted
                ? say($lang, "skills_admitted")
                : say($lang, "skills_not_admitted")}
              used={skill.pinned_by.length > 0
                ? fill(say($lang, "skills_used_by"), { n: String(skill.pinned_by.length) })
                : say($lang, "skills_used_never")}
              changed={changed(skill)}
              wire={{
                type: "button",
                disabled: place.at === null,
                onclick: () => {
                  open(place.at);
                },
              }}
            />
          </li>
        {/each}
      </ul>
    {:else}
      <!-- An empty shelf says what to do next: where a skill goes and
           how a building's runs come to read it. The audit and usage
           panels a skill will carry are not drawn until the city
           answers for them, so no empty frame passes for data. -->
      <p class="text-text-faint">{say($lang, "skills_empty_next")}</p>
    {/if}
    {#if answer.missing.length > 0}
      <p class="mt-base text-note text-alert">
        {say($lang, "skills_missing")}: {answer.missing.join(" · ")}
      </p>
    {/if}
  {/if}
</div>
