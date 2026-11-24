// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { UnansweredProps } from "./unanswered";
import Look from "./unanswered.look.solid";

export default function Unanswered(props: UnansweredProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  return (
    <Look
      said={fill(say(lang(), "answer_unavailable"), { query: props.query })}
      reason={props.reason ?? undefined}
      retry={{
        label: say(lang(), "link_retry"),
        press: () => {
          u.conn.asking.refresh(props.asked);
        },
      }}
    />
  );
}
