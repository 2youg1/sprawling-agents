// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, For, onCleanup, Show } from "solid-js";
import type {
  AccountsLook,
  ControlLook,
  FieldLook,
} from "../../../../src/views/setup/providers/accounts";
import "./accounts.solid.css";

function AccountControl(props: { readonly control: ControlLook }) {
  const help = createUniqueId();
  return (
    <span
      ref={(node) => {
        const stop = props.control.hold?.(node);
        if (typeof stop === "function") onCleanup(stop);
      }}
    >
      <button
        type="button"
        data-control=""
        class="swap-account-control"
        aria-disabled={props.control.why === undefined ? undefined : "true"}
        aria-describedby={props.control.why === undefined ? undefined : help}
        onClick={() => {
          if (props.control.why === undefined) props.control.press();
        }}
      >
        {props.control.label}
      </button>
      <Show when={props.control.why}>
        {(why) => (
          <span class="swap-account-quiet" id={help}>
            {why()}
          </span>
        )}
      </Show>
    </span>
  );
}

function AccountField(props: { readonly field: FieldLook }) {
  const help = createUniqueId();
  return (
    <label class="swap-account-field">
      <span>{props.field.label}</span>
      <input
        type={props.field.secret ? "password" : "text"}
        class={{ "swap-account-mono": props.field.mono }}
        value={props.field.value}
        disabled={props.field.disabled}
        aria-describedby={props.field.help === undefined ? undefined : help}
        onInput={(event) => props.field.input(event.currentTarget.value)}
      />
      <Show when={props.field.help}>{(words) => <small id={help}>{words()}</small>}</Show>
    </label>
  );
}

export default function AlternateAccounts(look: AccountsLook) {
  const retriesId = createUniqueId();
  const problem = createUniqueId();
  const standing = () => look.fields.filter((field) => !field.folded);
  const folded = () => look.fields.filter((field) => field.folded);
  return (
    <section class="swap-accounts">
      <h4
        tabindex={-1}
        ref={(node) => {
          const stop = look.holdHeading(node);
          if (typeof stop === "function") onCleanup(stop);
        }}
      >
        {look.heading}
      </h4>
      <p class="swap-account-quiet">{look.order}</p>
      <Show when={look.legacy}>{(words) => <p>{words()}</p>}</Show>
      <Show when={look.rows.length > 0}>
        <ol class="swap-account-rows">
          <For each={look.rows} keyed={(row) => row.id}>
            {(row) => (
              <li class="swap-account-row">
                <span class="swap-account-place">{row().place}</span>
                <span class="swap-account-mono">{row().id}</span>
                <span class={{ "swap-account-alert": row().wanting }}>{row().key}</span>
                <span class="swap-account-controls">
                  <For each={row().controls} keyed={(control) => control.key}>
                    {(control) => <AccountControl control={control()} />}
                  </For>
                </span>
              </li>
            )}
          </For>
        </ol>
      </Show>
      <Show when={look.forget}>
        {(forget) => (
          <p class="swap-account-controls">
            <span class="swap-account-quiet">{forget().text}</span>
            <button
              type="button"
              data-control=""
              class="swap-account-control"
              onClick={() => forget().press()}
            >
              {forget().label}
            </button>
          </p>
        )}
      </Show>
      <Show when={look.retries}>
        {(retries) => (
          <fieldset class="swap-account-retries">
            <legend>{retries().label}</legend>
            <For each={retries().options} keyed={(option) => option.value}>
              {(option) => (
                <label>
                  <input
                    type="radio"
                    name={retriesId}
                    checked={retries().held === option().value}
                    disabled={option().why !== undefined}
                    onChange={() => retries().pick(option().value)}
                  />
                  {option().label}
                </label>
              )}
            </For>
            <Show when={retries().fallback}>{(words) => <small>{words()}</small>}</Show>
          </fieldset>
        )}
      </Show>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (look.save.why === undefined && !look.save.loading) look.save.press();
        }}
      >
        <h5>{look.title}</h5>
        <For each={standing()} keyed={(field) => field.name}>
          {(field) => <AccountField field={field()} />}
        </For>
        <details>
          <summary data-control="">{look.more}</summary>
          <For each={folded()} keyed={(field) => field.name}>
            {(field) => <AccountField field={field()} />}
          </For>
        </details>
        <p class="swap-account-quiet">{look.route}</p>
        <Show when={look.note}>
          {(words) => (
            <p role="alert" class="swap-account-alert">
              {words()}
            </p>
          )}
        </Show>
        <Show when={look.refused}>
          {(error) => (
            <p role="alert" class="swap-account-alert">
              {error().code} {"·"} {error().recovery}
            </p>
          )}
        </Show>
        <span class="swap-account-controls">
          <button
            type="submit"
            data-control=""
            class="swap-account-control"
            data-control-state={look.save.loading ? "pending" : undefined}
            aria-busy={look.save.loading ? "true" : "false"}
            aria-disabled={look.save.why === undefined && !look.save.loading ? undefined : "true"}
            aria-describedby={look.save.why === undefined ? undefined : problem}
          >
            {look.save.label}
          </button>
          <Show when={look.save.why}>
            {(why) => (
              <span class="swap-account-quiet" id={problem}>
                {why()}
              </span>
            )}
          </Show>
          <Show when={look.cancel}>
            {(cancel) => (
              <button
                type="button"
                data-control=""
                class="swap-account-control"
                onClick={() => cancel().press()}
              >
                {cancel().label}
              </button>
            )}
          </Show>
        </span>
      </form>
    </section>
  );
}
