cargo build --bin powergrid-server
cargo build --bin powergrid-client
cargo build --bin powergrid-bot

trap 'kill $(jobs -p) 2>/dev/null' EXIT

cargo run --bin powergrid-client -- --windowed --name "brent" --color "red" --server localhost &
sleep 3

BOTS=("brad-bot:blue" "nick-bot:green" "jamie-bot:yellow")

for bot in "${BOTS[@]}"; do
    IFS=':' read -r name color <<< "$bot"
    cargo run --bin powergrid-bot -- --name "$name" --color "$color" --server localhost &
done

RUST_LOG=info cargo run --bin powergrid-server
