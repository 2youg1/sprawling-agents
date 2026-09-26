<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What this building can do: the city's library and the building's
  // own shelf, in one list with the shelf named on each row. A row says
  // whether the reading room admits it and how many runs were frozen
  // with it; opening one shows the document itself, through the same
  // file view the tree opens a file with.
  //
  // Where a shelved skill lives, in the two readings this page needs at
  // once: the word for which shelf it is on, and the address to open.
  //
  // **One function for both, because an external shelf has no
  // address.** A skill mounted from a directory outside the city is
  // named by which shelf it came from and by its path inside that
  // shelf; there is no city address to hand `Query::Document`, and
  // inventing one that looks like an address would be a lie the file
  // view would then fail to open. Returning the word and the address
  // together is what stops a caller from having a shelf it can name and
  // an address it cannot.
  import type { Lang } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import type { Address, SkillLine, SkillShelf } from "../../wire";

  interface Place {
    // Already in the person's language.
    readonly word: string;
    // Absent for an external shelf, which is not in this city.
    readonly at: Address | null;
  }

  function placeOf(shelf: SkillShelf, lang: Lang): Place {
    if ("building" in shelf) return { word: say(lang, "skills_shelf"), at: shelf.building };
    if ("library" in shelf) return { word: say(lang, "skills_library"), at: shelf.library };
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
    if ("building" in shelf) return `${shelf.building}·${skill.name}`;
    if ("library" in shelf) return `${shelf.library}·${skill.name}`;
    return `external:${String(shelf.external.index)}·${skill.name}`;
  }
</script>

<script lang="ts">
  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import type { Query } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import type { Picked } from "./tree.svelte";

  interface Props {
    readonly building: Address;
    readonly onPick: (picked: Picked) => void;
  }

  const { building, onPick }: Props = $props();

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
  <h2 class="mb-base text-heading font-heading">{say($lang, "bld_skills")}</h2>
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if answer === undefined}
    <p class="text-text-faint">…</p>
  {:else}
    {#if answer.skills.length > 0}
      <ul>
        {#each answer.skills as skill (skillId(skill))}
          {const place = placeOf(skill.shelf, $lang)}
          <li class="border-b border-edge">
            <button
              type="button"
              disabled={place.at === null}
              class="flex w-full min-w-0 items-baseline gap-base py-snug text-left text-note hover:text-text disabled:cursor-default disabled:hover:text-inherit"
              onclick={() => {
                open(place.at);
              }}
            >
              <span class="shrink-0 font-mono text-text-quiet">{skill.name}</span>
              <span class="shrink-0 text-text-faint">{place.word}</span>
              <span class="min-w-0 flex-1 truncate text-text-faint">{skill.disclosure}</span>
              <!-- The two trailing qualifiers shrink; they do not hold
                  their width against the row. Five cells were `shrink-0`
                  except the one in the middle, so once the four fixed
                  ones and their gaps passed the pane's width the last of
                  them was painted outside the button and over the column
                  beside it. The name and the shelf keep their width
                  because they are what identifies the row; whether it is
                  admitted and who pinned it are qualifiers, and a
                  qualifier that has to be cut short is still read. -->
              <span class="min-w-0 truncate text-text-faint">
                {skill.admitted ? say($lang, "skills_admitted") : say($lang, "skills_not_admitted")}
              </span>
              <span class="min-w-0 truncate text-text-faint">
                {skill.pinned_by.length > 0
                  ? fill(say($lang, "skills_used_by"), { n: String(skill.pinned_by.length) })
                  : say($lang, "skills_used_never")}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-text-faint">{say($lang, "skills_empty")}</p>
    {/if}
    {#if answer.missing.length > 0}
      <p class="mt-base text-note text-alert">
        {say($lang, "skills_missing")}: {answer.missing.join(" · ")}
      </p>
    {/if}
  {/if}
</div>
