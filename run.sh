cargo build --bin powergrid-server

trap 'kill $(jobs -p) 2>/dev/null' EXIT

NUM_PLAYERS=${1:-3}

PLAYERS=("brent:red" "brad:blue" "nick:green" "jamie:yellow" "niki:purple" "jodi:white")

for i in $(seq 0 $((NUM_PLAYERS - 1))); do
    IFS=':' read -r name color <<< "${PLAYERS[$i]}"
    cargo run --bin powergrid-client -- --name "$name" --color "$color" --url ws://localhost:3000/ws &
    sleep 1
done

RUST_LOG=info cargo run --bin powergrid-server
