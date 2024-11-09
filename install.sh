#!/bin/bash

cargo build --release --bin recpt3
install target/release/recpt3 /usr/local/bin

cargo build --release --bin checksignal
install target/release/checksignal /usr/local/bin

cargo build --release --bin ts_splitter
install target/release/ts_splitter /usr/local/bin
