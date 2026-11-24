// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { openDocument } from "../inspect/open";
import { searchLookOf } from "./search";
import Look from "./search.look.solid";

export interface PortProps {
  readonly under: Address;
  readonly titleId: string;
  readonly onClose: () => void;
}

export default function FinderSearch(props: PortProps) {
  const uid = createUniqueId();
  const u = ui();
  const lang = storeValue(u.lang);
  const [text, setText] = createSignal("");
  const [cursor, setCursor] = createSignal(0);
  const term = createMemo(() => text().trim());
  const answer = storeValue(() =>
    u.conn.asking.ask({ find: { under: props.under, text: term() } }),
  );
  const found = createMemo(() => {
    const result = answer();
    return result !== undefined &&
      "find" in result &&
      result.find.under === props.under &&
      result.find.text === term()
      ? result.find
      : undefined;
  });
  const look = createMemo(() =>
    searchLookOf(
      {
        under: props.under,
        titleId: props.titleId,
        uid,
        text: text(),
        cursor: Math.max(0, Math.min(cursor(), (found()?.paths.length ?? 0) - 1)),
        found: found(),
        lang: lang(),
      },
      {
        type: (typed) => {
          setText(typed);
          setCursor(0);
        },
        point: setCursor,
        open: (path) => {
          openDocument({ building: props.under, path, version: null });
          props.onClose();
        },
      },
    ),
  );
  return <Look {...look()} />;
}
