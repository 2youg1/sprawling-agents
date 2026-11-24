// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";

import { readDocument } from "../../core/document";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import EmptyState from "../parts/empty.solid";
import { CITY_CONFIG } from "../settings/files";

interface DisclosureProps {
  readonly region: { readonly "aria-label": string };
  readonly toggle: string;
  readonly path: string;
  readonly text: string | undefined;
}

export function TomlDisclosure(props: DisclosureProps) {
  return (
    <aside aria-label={props.region["aria-label"]} class="min-w-0">
      <details class="rounded-card bg-chrome/60 px-base py-tight">
        <summary class="cursor-pointer text-label text-text-quiet transition-colors duration-short ease-leave hover:text-text hover:ease-arrive">
          {props.toggle}{" "}
          <code class="ml-snug font-mono text-note text-text-faint">{props.path}</code>
        </summary>
        <Show
          when={props.text !== undefined}
          fallback={<EmptyState missing={"setup_toml_unread"} />}
        >
          <pre class="overflow-x-auto py-snug font-mono text-note text-text-quiet">
            {props.text}
          </pre>
        </Show>
      </details>
    </aside>
  );
}

export default function Toml() {
  const u = ui();
  const lang = storeValue(u.lang);
  const file = storeValue(u.conn.asking.ask({ document: { at: CITY_CONFIG } }));
  const source = createMemo(() => {
    const read = readDocument(file());
    return read.kind === "held" && read.value.text !== "" ? read.value.text : undefined;
  });
  return (
    <TomlDisclosure
      region={{ "aria-label": say(lang(), "setup_toml") }}
      toggle={say(lang(), "setup_toml_toggle")}
      path={CITY_CONFIG}
      text={source()}
    />
  );
}
