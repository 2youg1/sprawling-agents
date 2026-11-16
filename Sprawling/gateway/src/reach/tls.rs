// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Once;

static INSTALLED: Once = Once::new();

#[expect(
    clippy::disallowed_methods,
    reason = "the one place the TLS backend is installed"
)]
pub(crate) fn install_provider() {
    INSTALLED.call_once(|| {
        if let Err(offered) = rustls::crypto::aws_lc_rs::default_provider().install_default() {
            drop(offered);
        }
    });
}
