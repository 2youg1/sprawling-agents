// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, For, Show, untrack } from "solid-js";
import { QUERIES } from "../core/asking";
import { createBuilding, type Template } from "../core/commands";
import { fill, say } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import { TemplateName, type Address, type AxError } from "../wire";
import { textBoxWire } from "./building/text_box";
import TextBox from "./building/text_box.look.solid";
import Button from "./parts/button.solid";
import Field from "./parts/field.solid";
import { noticeTitle } from "./parts/notice_title";
import Page from "./parts/page.solid";
import type { Rank } from "./parts/page";
import Segmented from "./parts/segmented.solid";
import { fillable, OFFERED, raisingOf, sourceOf, sourceRefusal } from "./raise";

export interface RaiseProps {
  readonly address?: Address | undefined;
  readonly rank?: Rank | undefined;
}

interface Sent {
  readonly addr: Address;
  readonly before: AxError | null;
  readonly importing: boolean;
}

export default function Raise(props: RaiseProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const city = storeValue(u.conn.asking.ask(QUERIES.city));
  const [typed, setTyped] = createSignal<string>(untrack(() => props.address ?? ""));
  const [template, setTemplate] = createSignal<Exclude<Template, "hall">>("minimal");
  const [rules, setRules] = createSignal("");
  const [source, setSource] = createSignal("");
  const [edited, setEdited] = createSignal(false);
  const [sent, setSent] = createSignal<Sent | null>(null);
  const [refusal, setRefusal] = createSignal<AxError | null>(null);
  const [offered, setOffered] = createSignal<string | null>(null);
  const [answered, setAnswered] = createSignal(false);
  let addressGroup: HTMLDivElement | undefined;

  const buildings = createMemo(() => {
    const reply = city();
    return reply !== undefined && "city" in reply ? reply.city.buildings : [];
  });
  const raising = createMemo(() => raisingOf(typed(), buildings()));
  const target = createMemo(() => {
    const choice = raising();
    return choice.kind === "build" || choice.kind === "adopt" ? choice.addr : null;
  });
  createEffect(
    () => [target(), template()] as const,
    ([at, choice]) => {
      setOffered(null);
      setAnswered(false);
      setEdited(false);
      setRules("");
      if (at === null) return;
      return u.conn.asking
        .ask({ template: { template: TemplateName.make(choice), addr: at } })
        .subscribe((reply) => {
          setOffered(reply !== undefined && "template" in reply ? reply.template.rules : null);
          setAnswered(reply !== undefined);
        });
    },
  );

  const ready = () => target() !== null && (edited() || answered());
  const importOf = () => sourceOf(source());
  const importing = () => sent()?.importing ?? importOf() !== null;
  const sourceWhy = () => sourceRefusal(source());
  createEffect(offered, (text) => {
    if (text !== null && !edited()) setRules(text);
  });
  createEffect(
    () => [sent(), buildings(), belief().refusal] as const,
    ([held, available, error]) => {
      if (held === null) return;
      if (available.some((each) => each.addr === held.addr && !each.adoptable)) {
        setSent(null);
        u.go({ kind: "building", address: held.addr });
      } else if (error !== null && error !== held.before) {
        setRefusal(error);
        setSent(null);
      }
    },
  );

  function raise(): void {
    const at = target();
    if (at === null || !ready() || sent() !== null || sourceWhy() !== null) return;
    setRefusal(null);
    const before = belief().refusal;
    const from = importOf();
    if (u.send(createBuilding(at, template(), edited() ? rules() : null, from))) {
      setSent({ addr: at, before, importing: from !== null });
      u.conn.asking.refresh(QUERIES.city);
    }
  }

  const fills = () => fillable(buildings(), typed());
  const note = (): string | undefined => {
    if (importOf() !== null && sourceWhy() === null) return say(lang(), "raise_will_import");
    switch (raising().kind) {
      case "blank":
        return say(lang(), "raise_address_help");
      case "refused":
        return undefined;
      case "build":
        return say(lang(), "raise_will_build");
      case "adopt":
        return say(lang(), "raise_will_adopt");
    }
  };
  const addressField = () => {
    const help = note();
    const choice = raising();
    return {
      ...(help === undefined ? {} : { help }),
      ...(choice.kind === "refused" ? { error: say(lang(), choice.why) } : {}),
    };
  };
  function refusalOfSource() {
    const reason = sourceWhy();
    return reason === null ? {} : { error: say(lang(), reason) };
  }
  function blocked() {
    if (sent() !== null) return {};
    if (target() === null) return { why: say(lang(), "raise_needs_address") };
    const reason = sourceWhy();
    if (reason !== null) return { why: say(lang(), reason) };
    return ready() ? {} : { why: say(lang(), "raise_rules_waiting") };
  }

  return (
    <Page title={say(lang(), "raise_title")} note={say(lang(), "raise_note")} rank={props.rank}>
      <form
        class="flex min-w-0 flex-col gap-wide"
        onSubmit={(event) => {
          event.preventDefault();
          raise();
        }}
      >
        <div
          class="flex flex-col gap-snug"
          ref={(node) => {
            addressGroup = node;
          }}
        >
          <Field
            label={say(lang(), "raise_address")}
            value={typed()}
            mono
            placeholder={say(lang(), "raise_address_placeholder")}
            {...addressField()}
            onInput={setTyped}
          />
          <Show when={fills().length > 0}>
            <div
              class="flex flex-wrap items-center gap-snug"
              role="group"
              aria-label={say(lang(), "raise_waiting")}
            >
              <span class="text-note text-text-quiet">{say(lang(), "raise_waiting")}</span>
              <For each={fills()}>
                {(folder) => (
                  <Button
                    label={folder}
                    onPress={() => {
                      setTyped(folder);
                      addressGroup?.querySelector("input")?.focus();
                    }}
                  />
                )}
              </For>
            </div>
          </Show>
        </div>
        <Field
          label={say(lang(), "raise_source_label")}
          value={source()}
          mono
          placeholder={say(lang(), "raise_source_placeholder")}
          help={say(lang(), "raise_source_help")}
          {...refusalOfSource()}
          onInput={setSource}
        />
        <div class="flex flex-col gap-snug">
          <span class="text-note text-text-quiet">{say(lang(), "raise_template")}</span>
          <Segmented
            label={say(lang(), "raise_template")}
            options={OFFERED.map((value) => ({
              value,
              label: say(lang(), `raise_template_${value}`),
            }))}
            held={template()}
            onPick={setTemplate}
          />
          <p class="text-note text-text-faint">
            {say(lang(), `raise_template_${template()}_note`)}
          </p>
        </div>
        <Show when={target()}>
          <label class="flex flex-col gap-snug">
            <span class="text-note text-text-quiet">{say(lang(), "raise_rules")}</span>
            <TextBox
              size="document"
              wire={textBoxWire(rules(), undefined, refusal() !== null, (value) => {
                setRules(value);
                setEdited(true);
              })}
            />
            <span class="text-note text-text-faint">
              {say(lang(), edited() ? "raise_rules_edited" : "raise_rules_note")}
            </span>
          </label>
        </Show>
        <Show when={refusal()}>
          {(rejected) => (
            <p class="text-note text-alert" role="alert">
              {fill(say(lang(), "raise_refused"), {
                why: noticeTitle(lang(), rejected().code, rejected().subject),
                fix: rejected().recovery,
              })}
            </p>
          )}
        </Show>
        <div class="flex flex-col items-start gap-snug">
          <Button
            type="submit"
            tone="primary"
            loading={sent() !== null}
            label={say(
              lang(),
              importing()
                ? "raise_import"
                : raising().kind === "adopt"
                  ? "raise_adopt"
                  : "raise_build",
            )}
            {...blocked()}
          />
          <Show when={sent()}>
            <p class="text-note text-text-quiet" role="status">
              {say(lang(), importing() ? "raise_importing" : "raise_raising")}
            </p>
          </Show>
        </div>
      </form>
    </Page>
  );
}
