-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Sprawling.wire.spec.Reception.Admission

/-!
## 8-95a The local workspace door

The registered directory belongs to the person. A workspace exposes that directory to RefRain through a paired loopback listener. This keeps direct filesystem access out of the remote listener, commands available to agents, and city addresses. Under the same entry policy as other HTTP doors, `POST /workspace` takes a tagged JSON request for listing, registration, opening, reading bytes, progress and classification, or explicit relinking. `POST /workspace/save` takes the workspace ID, record ID and baseline digest in the query and the replacement as an octet stream. Replies are JSON except for file bytes, which travel as `application/octet-stream` only after the requested content version is established. A directory response carries up to 128 entries and a continuation for the same directory snapshot. When native notifications are available it carries a revision. Watching with the same revision waits for a change, watcher loss, or a 30-second renewal. No paths from a native notification travel in the reply. The listener never caches the response.

Registration names an absolute path on the city host. Later operations take the registration ID and a relative file path or record ID. Opening yields a record ID, path, content digest, optional category and one-based page with a normalized page offset, and last opened time. A request for a changed version yields `E_VERSION_CONFLICT`. The app specification defines file identity and authorization boundaries.
-/

namespace Wire.Server.Workspace

def admits (loopback : Bool) (pairing : Wire.Reception.Admission.Pairing) : Bool :=
  match pairing with
  | .Held => loopback
  | .Absent => false

theorem remote_refused (pairing : Wire.Reception.Admission.Pairing) :
    admits false pairing = false := by cases pairing <;> rfl

theorem absent_refused (loopback : Bool) : admits loopback .Absent = false := rfl

end Wire.Server.Workspace
