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
        serving::Keyed::Unshown(_) => {}
        serving::Keyed::Adopted(_) => {
            println!("    key      the one you configured; this city will ask for it");
            println!();
        }
        // Shown here and nowhere else a person reads; the key file this
        // account alone reads holds it for programs on this machine.
        // Never inside an address: an address goes into the browser's
        // history, its bookmarks and screenshots, and the page reads no
        // key from it (`crates/sprawling/spec/Keying.lean` §8-22).
        serving::Keyed::Minted(code) => {
            println!("    key      {code}");
            println!();
            println!("  This address reaches past this machine, so the city minted a key.");
            println!("  It is shown once and replaced the next time you start; a program on");
            println!("  another machine presents it with --token.");
            println!();
        }
    }
    println!("  Open the WebUI in a browser. Ctrl-C stops the city.");
    println!();
}

/// Who reads the lines this city prints, which decides how a person
/// pairs a browser with it.
pub(super) enum Reader {
    /// The line console: `/web` is typed here.
    LineConsole,
    /// Nobody at a console; usually a log.
    Log,
}

/// How a browser is paired with a city that has no face of its own. The
/// pairing code is not printed: this output is usually a log, a code in
/// it stays good until its first guess, and it is never printed again
/// when a guess replaces it (`crates/sprawling/spec/Firstrun.lean` §8-8).
pub(super) fn print_pairing(reader: Reader) {
    match reader {
        Reader::LineConsole => {
            println!("  This terminal reads one line at a time. `/help` lists what it takes,");
            println!("  and `/serving` says where this city listens and what is running in it.");
            println!("  `/web` opens a browser on this machine already paired with the city.");
        }
        Reader::Log => {
            println!("  No pairing code is printed here, because this output is usually kept");
            println!("  as a log. To pair a browser on this machine, serve the city with --open,");
            println!("  which opens a page already paired, or with --console and type `/web`.");
        }
    }
    println!();
}
