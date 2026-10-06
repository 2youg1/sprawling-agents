// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(super) fn read_http_request(
    stream: std::net::TcpStream,
) -> (
    std::io::BufReader<std::net::TcpStream>,
    String,
    serde_json::Value,
) {
    use std::io::{BufRead as _, Read as _};
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut reader = std::io::BufReader::new(stream);
    let mut headers = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "complete HTTP headers"
        );
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse::<usize>().unwrap();
        }
        headers.push_str(&line);
    }
    let mut request = vec![0; length];
    reader.read_exact(&mut request).unwrap();
    (reader, headers, serde_json::from_slice(&request).unwrap())
}
