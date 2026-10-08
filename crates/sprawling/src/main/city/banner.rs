// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The banner a city with no face of its own prints once it listens
//! (`crates/sprawling/spec/Firstrun.lean` §8-8): a harness reads the URL
//! from it, so it stays plain lines on standard output.

use sprawling::serving;

/// What a person reads once the city listens: where it is, where to
/// open it, what client it serves, and the key when one was minted.
pub(super) fn print_banner(
    city: &std::path::Path,
    url: &str,
    client_line: &str,
    keyed: &serving::Keyed,
) {
    println!();
    println!("  sprawling is running.");
    println!();
    println!("    city     {}", city.display());
    println!("    WebUI    {url}");
    println!("    client   {client_line}");
    println!();
    match keyed {
        serving::Keyed::NothingToPresent => {}
        serving::Keyed::Adopted(_) => {
            println!("    key      the one you configured; this city will ask for it");
            println!();
        }
        // Shown here and nowhere else, for as long as this process
        // lives. Nothing writes it down, so a person who loses it stops
        // and starts the city again rather than looking for a file.
        serving::Keyed::Minted(code) => {
            println!("    key      {code}");
            println!();
            println!("  This address reaches past this machine, so the city minted a key.");
            println!("  It is shown once, kept nowhere, and replaced the next time you start.");
            println!("  Open:    {}/?token={code}", url.trim_end_matches('/'));
            println!();
        }
    }
    println!("  Open the WebUI in a browser. Ctrl-C stops the city.");
    println!();
}
