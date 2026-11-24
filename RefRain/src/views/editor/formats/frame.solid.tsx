// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import { framed } from "./framed";
interface Props {
  readonly page: string;
  readonly scheme: "authored" | "built";

  readonly title: string;
}

export type PortProps = Props;

export default function EditorFormatsFrame(props: Props) {
  const shown = createMemo(() => framed(props.page, props.scheme));
  return (
    <>
      <iframe
        class={"min-h-0 w-full flex-1 border-0"}
        sandbox={""}
        referrerpolicy={"no-referrer"}
        title={props.title}
        srcdoc={shown()}
      />
    </>
  );
}
