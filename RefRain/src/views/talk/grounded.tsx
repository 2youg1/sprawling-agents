// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Prose from "../prose";
import { GroundedLinks } from "./grounded_links";
import { groundedAnswer, type DocumentSources } from "./source_context";

export interface GroundedProps {
  readonly text: string;
  readonly sources: DocumentSources;
}

export default function Grounded(props: GroundedProps) {
  const lang = storeValue(ui().lang);
  const answer = createMemo(() => groundedAnswer(props.text, props.sources));
  return (
    <>
      <Show when={answer().words !== ""}>
        <Prose text={answer().words} />
      </Show>
      <GroundedLinks citations={answer().citations} proposal={answer().proposal} lang={lang()} />
    </>
  );
}
