// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Match, Show, Switch } from "solid-js";

import { QUERIES } from "../../core/asking";
import { say, type Key } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Button from "../parts/button.solid";
import Field from "../parts/field.solid";
import Unanswered from "../parts/unanswered.solid";
import { identityEditor } from "./identity_edit";
import ImportLogin from "./import_view";
import SaveCard from "./save_card_view";

interface Person {
  readonly user_id: string | null;
  readonly imported_from: string | null;
  readonly about: string;
}

function samePerson(a: Person, b: Person): boolean {
  return (
    (a.user_id?.trim() || null) === (b.user_id?.trim() || null) &&
    a.imported_from === b.imported_from &&
    a.about === b.about
  );
}

export default function You() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUERIES.identity));
  const word = (key: Key) => say(lang(), key);
  const received = () => {
    const answer = asked();
    return answer !== undefined && "identity" in answer ? answer.identity : undefined;
  };
  const stated = () => {
    const identity = received();
    return identity !== undefined && "stated" in identity ? identity.stated : undefined;
  };
  const unreadable = () => {
    const identity = received();
    return identity !== undefined && "unreadable" in identity ? identity.unreadable : undefined;
  };
  const unavailable = () => {
    const answer = asked();
    return answer !== undefined && "unavailable" in answer ? answer.unavailable : undefined;
  };
  const person = identityEditor<Person>(
    () => {
      const read = stated();
      return read === undefined
        ? undefined
        : {
            base: read.preferences_text,
            text: {
              user_id: read.user_id ?? null,
              imported_from: read.imported_from ?? null,
              about: read.about,
            },
          };
    },
    (person) => {
      const user_id = person.user_id?.trim() || null;
      return {
        person: {
          ...person,
          user_id,
          imported_from: user_id === null ? null : person.imported_from,
        },
      };
    },
    samePerson,
  );
  const mayor = identityEditor(
    () => {
      const read = stated();
      return read === undefined ? undefined : { base: read.mayor_text, text: read.mayor ?? "" };
    },
    (text) => ({ mayor: { name: text.trim() || null } }),
    (a, b) => a.trim() === b.trim(),
  );

  function change(withPart: Partial<Person>): void {
    const text = person.actual();
    if (text !== undefined) person.edit({ ...text, ...withPart });
  }

  function Resolve(props: {
    readonly visible: boolean;
    readonly body: string;
    readonly onRebase: () => void;
  }) {
    return (
      <Show when={props.visible}>
        <details>
          <summary data-control="surface">{word("setup_governed_compare")}</summary>
          <pre class="max-h-output overflow-auto whitespace-pre-wrap">{props.body}</pre>
          <Button label={word("setup_governed_rebase")} glyph="check" onPress={props.onRebase} />
        </details>
      </Show>
    );
  }

  return (
    <Switch>
      <Match when={unreadable()}>
        {(problem) => (
          <div class="asks flex max-w-talk flex-col gap-tight rounded-card bg-raised px-base py-snug">
            <span class="text-label font-label text-text">{word("you_unreadable")}</span>
            <p class="font-mono text-note text-text-quiet">
              {`${problem().document === "mayor" ? "MAYOR.md" : "PREFERENCES.md"}:${String(problem().line)} · ${problem().why}`}
            </p>
            <a
              href={toFragment({ kind: "setup", group: "run" })}
              class="w-fit text-label text-text-quiet underline hover:text-text"
            >
              {word("you_unreadable_open")}
            </a>
          </div>
        )}
      </Match>
      <Match when={stated()}>
        {(read) => (
          <div class="flex max-w-talk flex-col gap-base">
            <SaveCard
              title={word("you_person")}
              note={word("you_person_note")}
              settled={word("you_next_session")}
              saving={person.status()}
              onSave={person.save}
              onCheck={person.check}
              blocked={person.conflict() ? "setup_governed_compare" : undefined}
            >
              <Field
                label={word("you_user_id")}
                placeholder={word("you_user_id_default")}
                mono
                value={person.actual()?.user_id ?? ""}
                onInput={(user_id) => change({ user_id, imported_from: null })}
              />
              <ImportLogin
                onTake={(login, host) => change({ user_id: login, imported_from: host })}
              />
              <label class="flex flex-col gap-tight">
                <span class="text-note text-text-quiet">{word("you_about")}</span>
                <textarea
                  class="min-h-figure w-full rounded-control border border-edge-input bg-raised px-base py-snug text-body text-text"
                  value={person.actual()?.about ?? ""}
                  onInput={(event) => change({ about: event.currentTarget.value })}
                />
              </label>
              <Resolve
                visible={person.conflict()}
                body={read().preferences_text}
                onRebase={person.rebase}
              />
            </SaveCard>
            <SaveCard
              title={word("you_mayor")}
              note={word("you_mayor_note")}
              settled={word("you_next_session")}
              saving={mayor.status()}
              onSave={mayor.save}
              onCheck={mayor.check}
              blocked={mayor.conflict() ? "setup_governed_compare" : undefined}
            >
              <Field
                label={word("you_mayor_name")}
                placeholder={word("nav_mayor")}
                value={mayor.actual() ?? ""}
                onInput={mayor.edit}
              />
              <p class="text-note text-text-faint">{word("you_mayor_address")}</p>
              <Resolve
                visible={mayor.conflict()}
                body={read().mayor_text}
                onRebase={mayor.rebase}
              />
            </SaveCard>
          </div>
        )}
      </Match>
      <Match when={unavailable()}>
        {(problem) => <Unanswered query={problem().query} asked={QUERIES.identity} />}
      </Match>
    </Switch>
  );
}
