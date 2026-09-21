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

import type { Address, SkillLine, SkillShelf, SkillsAnswer } from "../../wire";
import { useSay, useUi } from "../../ui";
import type { Picked } from "./tree";

// Where a shelved skill lives, in the two readings this page needs at
// once: the word for which shelf it is on, and the address to open.
//
// **One function for both, because an external shelf has no address.**
// A skill mounted from a directory outside the city is named by which
// shelf it came from and by its path inside that shelf; there is no
// city address to hand `Query::Document`, and inventing one that looks
// like an address would be a lie the file view would then fail to
// open. Returning the word and the address together is what stops a
// caller from having a shelf it can name and an address it cannot.
interface Place {
  // Already in the person's language.
  readonly word: string;
  // Absent for an external shelf, which is not in this city.
  readonly at: Address | null;
}

function placeOf(shelf: SkillShelf, say: ReturnType<typeof useSay>): Place {
  if ("building" in shelf) return { word: say("skills_shelf"), at: shelf.building };
  if ("library" in shelf) return { word: say("skills_library"), at: shelf.library };
  return {
    word: say("skills_external", { n: String(shelf.external.index), path: shelf.external.path }),
    at: null,
  };
}

function Row(props: { readonly skill: SkillLine; readonly onOpen: (at: Address) => void }) {
  const say = useSay();
  const place = () => placeOf(props.skill.shelf, say);
  // A row this city cannot open is not a button. An external shelf is
  // read-only and lives outside every address this client can ask
  // about, so the row states where it came from and stops there.
  const open = () => {
    const at = place().at;
    if (at !== null) props.onOpen(at);
  };
  return (
    <li class="border-b border-edge">
      <button
        type="button"
        disabled={place().at === null}
        class="flex w-full min-w-0 items-baseline gap-base py-snug text-left text-note hover:text-text disabled:cursor-default disabled:hover:text-inherit"
        onClick={open}
      >
        <span class="shrink-0 font-mono text-text-quiet">{props.skill.name}</span>
        <span class="shrink-0 text-text-disabled">{place().word}</span>
        <span class="min-w-0 flex-1 truncate text-text-faint">{props.skill.disclosure}</span>
        {/* The two trailing qualifiers shrink; they do not hold their
            width against the row. Five cells were `shrink-0` except
            the one in the middle, so once the four fixed ones and
            their gaps passed the pane's width the last of them was
            painted outside the button and over the column beside it.
            The name and the shelf keep their width because they are
            what identifies the row; whether it is admitted and who
            pinned it are qualifiers, and a qualifier that has to be
            cut short is still read. `xtask render --survey` found
            this, at three widths and in both lightings, after the
            same defect had been repaired once in `parts/popover.tsx`
            - which is how a family of defects gets found rather than
            an instance. */}
        <span class="min-w-0 truncate text-text-disabled">
          {props.skill.admitted ? say("skills_admitted") : say("skills_not_admitted")}
        </span>
        <span class="min-w-0 truncate text-text-disabled">
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
                      onOpen={(at) => {
                        props.onPick({ at, kind: "file" });
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
