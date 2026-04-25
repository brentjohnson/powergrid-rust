cargo build --bin powergrid-server
cargo build --bin powergrid-client
cargo build --bin powergrid-bot

trap 'kill $(jobs -p) 2>/dev/null' EXIT

RUST_LOG=info cargo run --bin powergrid-server &
sleep 1

cargo run --bin powergrid-client -- --windowed --name "brent" --color "red" --server localhost &
sleep 3

BOTS=("brad-bot:blue" "nick-bot:green" "jamie-bot:yellow")

cargo run --bin powergrid-bot -- --name "brad-bot" --color "blue" --server localhost &
cargo run --bin powergrid-bot -- --name "nick-bot" --color "green" --server localhost &
cargo run --bin powergrid-bot -- --name "jamie-bot" --color "yellow" --server localhost

