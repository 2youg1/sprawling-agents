// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::process::ExitCode;
use std::sync::Arc;

use calyx::clock::Wall;
use calyx::console::mei;

fn main() -> ExitCode {
    let replay = replay_flag(std::env::args().skip(1));
    mei::verb(
        replay.as_deref(),
        Arc::new(|| accounting::Clock::now(&Wall)),
    )
}

fn replay_flag(mut args: impl Iterator<Item = String>) -> Option<String> {
    while let Some(arg) = args.next() {
        if arg == "--replay" {
            return args.next();
        }
        if let Some(path) = arg.strip_prefix("--replay=") {
            return Some(path.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn replay_flag_reads_the_next_argument() {
        let path = super::replay_flag(["--replay", "run.jsonl"].into_iter().map(str::to_owned));
        assert_eq!(path.as_deref(), Some("run.jsonl"));
    }
}
