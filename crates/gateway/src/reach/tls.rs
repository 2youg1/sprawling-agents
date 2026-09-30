// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The process's one TLS crypto backend (gateway-SPEC.md section 8-15).
//!
//! rustls asks the process for a default provider when a client is
//! built; reqwest is compiled with `rustls-no-provider`, so nothing
//! answers unless this module installed one first. The backend is
//! aws-lc-rs, the same library the remote door signs and seals with.

use std::sync::Once;

static INSTALLED: Once = Once::new();

/// Makes aws-lc-rs rustls's process-wide provider; later calls do nothing.
///
/// An `Err` from `install_default` hands back the provider this call
/// offered, because another was already the process default. This
/// binary compiles no other backend and clippy refuses any other call
/// to `install_default`, so that provider is this one, and the offered
/// copy is dropped with nothing left to do.
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
