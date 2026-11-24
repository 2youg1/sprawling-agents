// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, onCleanup, Show } from "solid-js";
import Button from "../../parts/button.solid";
import Field from "../../parts/field.solid";
import Notice from "../../parts/notice_view";
import Segmented from "../../parts/segmented.solid";
import type { AccountsLook, FieldLook } from "./accounts";

function DrawField(props: { readonly field: FieldLook }) {
  return (
    <Field
      label={props.field.label}
      value={props.field.value}
      onInput={props.field.input}
      kind={props.field.secret ? "password" : "text"}
      mono={props.field.mono}
      disabled={props.field.disabled}
      {...(props.field.help === undefined ? {} : { help: props.field.help })}
    />
  );
}

function reason(why: string | undefined): { readonly why?: string } {
  return why === undefined ? {} : { why };
}

export default function AccountsLookView(look: AccountsLook) {
  const standing = () => look.fields.filter((field) => !field.folded);
  const folded = () => look.fields.filter((field) => field.folded);
  return (
    <div class="flex min-w-0 flex-col gap-snug">
      <h4
        tabindex={-1}
        class="font-label text-text-quiet"
        ref={(node) => {
          const stop = look.holdHeading(node);
          if (typeof stop === "function") onCleanup(stop);
        }}
      >
        {look.heading}
      </h4>
      <p class="text-note text-text-faint">{look.order}</p>
      <Show when={look.legacy}>
        {(legacy) => <p class="text-note text-text-quiet">{legacy()}</p>}
      </Show>
      <Show when={look.rows.length > 0}>
        <ol class="flex flex-col gap-tight">
          <For each={look.rows} keyed={(row) => row.id}>
            {(row) => (
              <li class="flex min-w-0 flex-wrap items-center gap-snug text-note">
                <span class="w-figure text-right font-mono text-text-faint">{row().place}</span>
                <span class="font-mono text-text">{row().id}</span>
                <span class={row().wanting ? "text-alert" : "text-text-faint"}>{row().key}</span>
                <span class="flex flex-1 flex-wrap justify-end gap-tight">
                  <For each={row().controls} keyed={(control) => control.key}>
                    {(control) => (
                      <span
                        ref={(node) => {
                          const stop = control().hold?.(node);
                          if (typeof stop === "function") onCleanup(stop);
                        }}
                      >
                        <Button
                          label={control().label}
                          tone="quiet"
                          onPress={control().press}
                          {...reason(control().why)}
                        />
                      </span>
                    )}
                  </For>
                </span>
              </li>
            )}
          </For>
        </ol>
      </Show>
      <Show when={look.forget}>
        {(forget) => (
          <div class="flex flex-wrap items-center gap-snug text-note">
            <span class="text-text-quiet">{forget().text}</span>
            <Button tone="quiet" label={forget().label} onPress={forget().press} />
          </div>
        )}
      </Show>
      <Show when={look.retries}>
        {(retries) => (
          <div class="flex flex-col gap-tight">
            <span class="text-note text-text-quiet">{retries().label}</span>
            <Segmented
              label={retries().label}
              options={retries().options}
              held={retries().held}
              onPick={retries().pick}
            />
            <Show when={retries().fallback}>
              {(hint) => <p class="text-note text-text-faint">{hint()}</p>}
            </Show>
          </div>
        )}
      </Show>
      <form
        class="flex flex-col gap-snug"
        onSubmit={(event) => {
          event.preventDefault();
          look.save.press();
        }}
      >
        <p class="font-label text-text-quiet">{look.title}</p>
        <For each={standing()} keyed={(field) => field.name}>
          {(field) => <DrawField field={field()} />}
        </For>
        <details>
          <summary data-control="" class="text-note text-text-quiet">
            {look.more}
          </summary>
          <div class="flex flex-col gap-snug pt-snug">
            <For each={folded()} keyed={(field) => field.name}>
              {(field) => <DrawField field={field()} />}
            </For>
          </div>
        </details>
        <p class="text-note text-text-faint">{look.route}</p>
        <Show when={look.note}>
          {(note) => (
            <p role="alert" class="text-note text-alert">
              {note()}
            </p>
          )}
        </Show>
        <Show when={look.refused}>
          {(refusal) => (
            <Notice
              seat="inline"
              weight="alert"
              action={refusal().action}
              code={refusal().code}
              subject={refusal().subject}
              recovery={refusal().recovery}
            />
          )}
        </Show>
        <span class="flex flex-wrap gap-snug">
          <Button
            type="submit"
            tone="secondary"
            label={look.save.label}
            loading={look.save.loading}
            {...(look.save.why === undefined ? {} : { why: look.save.why })}
          />
          <Show when={look.cancel}>
            {(cancel) => <Button tone="quiet" label={cancel().label} onPress={cancel().press} />}
          </Show>
        </span>
      </form>
    </div>
  );
}
