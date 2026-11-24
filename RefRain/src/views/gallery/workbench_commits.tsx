// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { ui } from "../../ui";
import { Commits } from "../world/commits_view";
import { answering, PICKED, recordsAt, ROOM } from "./benched";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

export default function WorkbenchCommitCase() {
  const records = recordsAt(ui().now());
  return (
    <GalleryCase label="workbench · a picked commit's identity in the commits pane">
      <Stand
        link={{ kind: "live", city: "sprawling" }}
        unread={[]}
        waiting={[]}
        answers={answering}
        commit={PICKED}
        records={records}
      >
        {() => (
          <div class="flex h-[520px] w-[440px] flex-col">
            <Commits here={ROOM} />
          </div>
        )}
      </Stand>
    </GalleryCase>
  );
}
