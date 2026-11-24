// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Show, Switch } from "solid-js";
import { readAnswer } from "../../core/answered";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { HeldSkill, Query, SkillAudit, SkillUse } from "../../wire";
import EmptyState from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";
import UsageExport from "../parts/usage_export.solid";
import UsageUses from "../parts/usage_uses.solid";
import type { UseRow } from "../parts/usage_uses";

const QUESTION: Query = { skill_usage: { skill: null } };

function rows(uses: readonly SkillUse[]): UseRow[] {
  return uses.map((one) => ({
    run: one.run,
    resident: one.resident ?? null,
    at: one.at,
    part: one.part,
    outcome: one.outcome,
  }));
}

export function auditWord(audit: SkillAudit): Key {
  switch (audit.state) {
    case "unaudited":
      return "usage_audit_none";
    case "stale":
      return "usage_reaudit";
    case "audited":
      switch (audit.verdict) {
        case "pass":
          return "usage_audit_pass";
        case "warn":
          return "usage_audit_warn";
        case "fail":
          return "usage_audit_fail";
        case "unreachable":
          return "usage_audit_none";
      }
  }
}

export function asksAgain(held: HeldSkill): boolean {
  return held.audit.state === "stale";
}

export default function SkillUsage() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUESTION));
  const read = createMemo(() =>
    readAnswer(asked(), (answer) => ("skill_usage" in answer ? answer.skill_usage : undefined)),
  );
  const skills = () => {
    const answer = read();
    return answer.kind === "held" ? answer.value.skills : [];
  };
  const used = createMemo(() => skills().filter((skill) => skill.uses.length > 0));
  const unused = createMemo(() => skills().filter((skill) => skill.uses.length === 0));
  const missing = () => {
    const answer = read();
    return answer.kind === "unavailable" ? answer : undefined;
  };
  const word = (key: Key) => say(lang(), key);

  return (
    <section class="flex min-w-0 flex-col gap-base">
      <div class="flex flex-wrap items-baseline justify-between gap-base">
        <h2 class="text-note text-text-faint">{word("usage_skills_title")}</h2>
        <UsageExport what="skills" />
      </div>
      <Switch
        fallback={
          <>
            <ul class="flex flex-col">
              <For each={[...used(), ...unused()]} keyed={(skill) => skill.name}>
                {(skill) => (
                  <li class="flex min-w-0 flex-col gap-tight border-b border-edge py-snug">
                    <span class="font-mono text-note text-text-quiet">{skill().name}</span>
                    <For each={skill().held} keyed={false}>
                      {(held) => (
                        <span
                          class={
                            asksAgain(held()) ? "text-note text-alert" : "text-note text-text-faint"
                          }
                        >
                          {word(auditWord(held().audit))}
                        </span>
                      )}
                    </For>
                    <Show when={skill().held.length === 0}>
                      <span class="text-note text-text-faint">{word("usage_off_shelf")}</span>
                    </Show>
                    <For each={skill().versions} keyed={(version) => version.digest}>
                      {(version) => (
                        <span class="truncate font-mono text-note text-text-faint">
                          {fill(word("usage_version"), {
                            digest: version().digest.slice(0, 12),
                            when: clock(lang(), version().at),
                            run: version().run,
                          })}
                        </span>
                      )}
                    </For>
                    <UsageUses uses={rows(skill().uses)} days={skill().per_day} />
                  </li>
                )}
              </For>
            </ul>
            <Show when={unused().length > 0}>
              <p class="text-note text-text-faint">
                {word("usage_never_list")}:{" "}
                {unused()
                  .map((skill) => skill.name)
                  .join(" · ")}
              </p>
            </Show>
          </>
        }
      >
        <Match when={missing()}>
          {(unavailable) => (
            <Unanswered
              query={unavailable().query}
              asked={QUESTION}
              reason={unavailable().reason}
            />
          )}
        </Match>
        <Match when={read().kind === "asking"}>
          <p class="text-text-faint">…</p>
        </Match>
        <Match when={skills().length === 0}>
          <EmptyState missing="usage_skills_none" seat="region" />
        </Match>
      </Switch>
    </section>
  );
}
