#!/bin/bash
cd "$(dirname "$0")/sender"
cargo build --release --quiet && ./target/release/screen-sender "$@"
