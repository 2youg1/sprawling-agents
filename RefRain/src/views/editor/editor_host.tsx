// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import "./editor_host.css";

interface Props {
  readonly ref?: ((node: HTMLDivElement) => void) | undefined;
}

export default function EditorHost(props: Props) {
  return <div class="editor" ref={(node) => props.ref?.(node)} />;
}
