// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId } from "solid-js";
import type { Address } from "../wire";
import FinderSearch from "./finder/search.solid";
import CommandDialog from "./palette/dialog";

export interface FinderProps {
  readonly under: Address;
  readonly onClose: () => void;
}

export default function Finder(props: FinderProps) {
  const title = createUniqueId();
  return (
    <CommandDialog
      seat="modal"
      named={{ "aria-labelledby": title }}
      backdrop="stays"
      onClose={props.onClose}
    >
      <FinderSearch under={props.under} titleId={title} onClose={props.onClose} />
    </CommandDialog>
  );
}
