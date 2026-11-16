// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

fn main() {

    let local = wire::Command::PutSecret {
        realm: "anthropic".to_owned(),
        name: "api".to_owned(),
        value: kernel::Sealed::new(Box::new("sk-not-a-real-key".to_owned())),
    };
    let _bytes = serde_json::to_string(&local);

    let _remote = wire::WireCommand::PutSecret {
        realm: "anthropic".to_owned(),
        name: "api".to_owned(),
        value: "sk-not-a-real-key".to_owned(),
    };
}
