// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import Monitor from "./monitor.solid";
import PartsBadge from "./parts/badge.solid";
import PartsCopy from "./parts/copy.solid";
import TabsCases from "./parts/tabs.solid";
import PartButton from "./parts/button.solid";
import PartField from "./parts/field.solid";
import PartPath from "./parts/path.solid";
import PartCombobox from "./parts/combobox.solid";
import PartTable from "./parts/table.solid";
import PartDialog from "./parts/dialog.solid";
import PartPopover from "./parts/popover.solid";
import PartTip, { PlacedTipCases } from "./parts/tip.solid";
import PartNotice from "./parts/notice.solid";
import PartSegmented from "./parts/segmented.solid";
import Monitored from "./monitored.solid";
import Timed from "./timed.solid";
import Lensed from "./lensed.solid";
import Recorded from "./recorded.solid";
import Resulted from "./resulted.solid";
import Streamed from "./streamed.solid";
import Tok from "./tok.solid";
import Read from "./read.solid";
import DialogDrawn from "./parts/dialog_drawn.solid";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { KbdCases } from "./parts/kbd_cases";
import Deciding from "./parts/decide.solid";
import Rfr from "./rfr.solid";
import Fmt from "./fmt.solid";
import G3 from "./g3.solid";
import Prop from "./prop.solid";
import Gd from "./gd.solid";
import G6 from "./g6.solid";
import LooksBuilding from "./looks_building.solid";
import LooksTalk from "./looks_talk.solid";
import LooksPages from "./looks_pages.solid";
import LooksParts from "./looks_parts.solid";
import { GalleryCase } from "./case";
import { DiagnosticView } from "../diagnostic_view";
import { AttachmentDrawer } from "../talk/attachment_drawer";
import { GroundedLinks } from "../talk/grounded_links";
import type { DraftAttachment } from "../talk/attachments";

function KbdMarks() {
  const lang = storeValue(ui().lang);
  return <KbdCases lang={lang()} />;
}

function DocumentActions() {
  const lang = storeValue(ui().lang);
  const source: DraftAttachment = {
    id: "example",
    workspace: "sample",
    file: "paper",
    version: "example",
    label: "paper.md",
    selection: { kind: "markdown", from: 0, to: 4, text: "text" },
  };
  return (
    <GalleryCase label="document actions · draft, citation and diagnostics">
      <AttachmentDrawer
        attachments={[source]}
        lang={lang()}
        storageFailure={false}
        remove={() => {}}
      />
      <GroundedLinks citations={[source]} lang={lang()} proposal={null} />
      <DiagnosticView
        error={{
          action: "read paper.md",
          code: "E_TIMEOUT",
          nearby: [],
          recovery: "open paper.md again",
          retry: "yes",
          subject: "paper.md",
        }}
        lang={lang()}
      />
    </GalleryCase>
  );
}

export const CONTROLS = [
  Monitor,
  PartsBadge,
  PartsCopy,
  TabsCases,
  PartButton,
  PartField,
  PartPath,
  PartCombobox,
  PartTable,
  PartDialog,
  PartPopover,
  PartTip,
  PartNotice,
  PartSegmented,
  DocumentActions,
  Monitored,
  Timed,
  Lensed,
  Recorded,
  Resulted,
  Streamed,
  Tok,
  Read,
  PlacedTipCases,
  DialogDrawn,
  KbdMarks,
  Deciding,
  Rfr,
  Fmt,
  G3,
  Prop,
  Gd,
  G6,
  LooksBuilding,
  LooksTalk,
  LooksPages,
  LooksParts,
] as const;
