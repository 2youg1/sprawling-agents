// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../core/lang";
import { ui } from "../../ui";
import Skeleton from "../parts/skeleton.solid";
import { storeValue } from "../../core/solid_store";

export type PortProps = Record<never, never>;

export default function MachineSkeleton() {
  const { lang } = ui();
  const $lang = storeValue(lang);
  return (
    <>
      <Skeleton label={say($lang(), "machine_checking")} rows={12} tall />
    </>
  );
}
