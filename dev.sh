#!/bin/sh
set -e
cd $HOME/github.com/loicbourgois/primus
# cargo fix --lib -p primus
cargo fmt
cargo run