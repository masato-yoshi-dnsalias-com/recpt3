#!/bin/bash

RUSTFLAGS="-L /lib -L /usr/lib -L /usr/local/lib" cargo build --release --bin recpt3
install target/release/recpt3 /usr/local/bin

RUSTFLAGS="-L /lib -L /usr/lib -L /usr/local/lib" cargo build --release --bin checksignal
install target/release/checksignal /usr/local/bin

RUSTFLAGS="-L /lib -L /usr/lib -L /usr/local/lib" cargo build --release --bin ts_splitter
install target/release/ts_splitter /usr/local/bin

RUSTFLAGS="-L /lib -L /usr/lib -L /usr/local/lib" cargo build --release --bin drop_check
install target/release/drop_check /usr/local/bin
