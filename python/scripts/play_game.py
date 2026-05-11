"""
Roll out one game and print the event log.

Usage:
    # Random vs random:
    python scripts/play_game.py

    # Trained model vs bots:
    python scripts/play_game.py --model runs/vs_bots/final_model --render

    # Bot vs bot (all seats Rust strategy):
    python scripts/play_game.py --all-bots
"""

import argparse
import json
import numpy as np

import powergrid_py  # type: ignore[import]

from powergrid_env import PowerGridAECEnv, RandomPolicy, RustBotPolicy
from powergrid_env.encoding import encode_observation, mask_from_info


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-players", type=int, default=4)
    parser.add_argument("--seed", type=int, default=None)
    parser.add_argument("--model", default=None, help="Path to a MaskablePPO checkpoint")
    parser.add_argument("--render", action="store_true")
    parser.add_argument("--all-bots", action="store_true", help="All seats use Rust bot")
    parser.add_argument("--difficulty", default="normal")
    args = parser.parse_args()

    env = PowerGridAECEnv(
        num_players=args.num_players,
        seed=args.seed,
        render_mode="ansi" if args.render else None,
    )
    env.reset(seed=args.seed)

    # Load optional model for seat 0.
    model = None
    if args.model:
        from sb3_contrib import MaskablePPO
        model = MaskablePPO.load(args.model)
        print(f"Loaded model from {args.model}")

    bot_policy = RustBotPolicy(difficulty=args.difficulty)
    random_policy = RandomPolicy()

    step_count = 0
    for agent in env.agent_iter():
        obs, reward, terminated, truncated, info = env.last()
        uuid = env._id_to_uuid.get(agent, agent)
        if terminated or truncated:
            action = None
        else:
            mask = info.get("action_mask", np.ones(env.action_space(agent).n, dtype=np.int8))
            if args.all_bots:
                action_json = env.game.bot_decide(uuid, args.difficulty)
                if action_json is None:
                    action = 0
                else:
                    from powergrid_env.encoding import action_json_to_id
                    state = json.loads(env.game.state_json())
                    action = action_json_to_id(action_json, state, uuid)
            elif model and agent == env.possible_agents[0]:
                action, _ = model.predict(obs, action_masks=mask, deterministic=True)
            else:
                action = bot_policy.act(
                    env.game, uuid,
                    json.loads(env.game.state_json()),
                    obs, mask,
                )

        env.step(action)
        step_count += 1

        if args.render and env.game and not env.game.is_terminal():
            print(env.render())
            print(f"  → agent={agent} action={action}")
            print()

    state = json.loads(env.game.state_json()) if env.game else {}
    print("\n=== Game Over ===")
    winner = env.game.winner() if env.game else None
    if winner:
        for p in state.get("players", []):
            if p["id"] == winner:
                print(f"Winner: {p['name']} (${p['money']}, {len(p.get('cities', []))} cities)")
                break
    print(f"Total steps: {step_count}")
    print("\nEvent log:")
    for msg in state.get("event_log", []):
        print(f"  {msg}")

    env.close()


if __name__ == "__main__":
    main()
