// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { readAnswer } from "../../core/answered";
import { QUERIES } from "../../core/asking";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import Glyph from "../parts/glyph.solid";
import Sandbox, { FACT } from "./sandbox_fact";

export default function Bounds(props: { readonly room: Address | null }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const governance = storeValue(u.conn.asking.ask(QUERIES.governance));
  const read = createMemo(() =>
    readAnswer(governance(), (held) => ("governance" in held ? held.governance : undefined)),
  );
  const words = createMemo(() => {
    const answer = read();
    switch (answer.kind) {
      case "asking":
        return "—";
      case "unavailable":
        return say(lang(), "facts_unreadable");
      case "held":
        return say(lang(), answer.value.autonomy === "owner" ? "autonomy_owner" : "autonomy_clerk");
    }
  });
  return (
    <>
      <span class={[FACT, read().kind === "unavailable" && "asks"]}>
        <Glyph name="gate" size="sm" />
        <span class="sr-only">{say(lang(), "facts_autonomy")}</span>
        {words()}
      </span>
      <Sandbox room={props.room} />
    </>
  );
}
