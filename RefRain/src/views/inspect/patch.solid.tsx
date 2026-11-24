// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, HunksAnswer } from "../../wire";
import { quoteLine, type CodeLine } from "../changes";
import { quoteInto } from "../talk/quoting";
import { lookOf } from "./patch";
import Look from "./patch.look.solid";

export interface PatchProps {
  readonly patch: HunksAnswer;
  readonly talk?: Address | undefined;
  readonly cursor?: number | null;
  readonly onCursor?: (line: CodeLine) => void;
}

export default function Patch(props: PatchProps) {
  const u = ui();
  const lang = storeValue(u.lang);

  function choose(line: CodeLine): void {
    if (props.talk === undefined) return;
    quoteInto(u.prefs, props.talk, quoteLine(props.patch, line));
    props.onCursor?.(line);
  }

  const look = createMemo(() =>
    lookOf(
      props.patch,
      props.cursor ?? null,
      {
        withheld: (n, reason) => fill(say(lang(), "run_withheld"), { n, reason }),
        folded: (n) => fill(say(lang(), "change_folded"), { n }),
        quote: (n) => fill(say(lang(), "change_line_quote"), { n }),
      },
      {
        href:
          props.talk === undefined ? undefined : toFragment({ kind: "talk", address: props.talk }),
        choose,
      },
    ),
  );
  return <Look {...look()} />;
}
