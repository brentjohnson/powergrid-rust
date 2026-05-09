"""
Train a single-agent MaskablePPO policy vs Rust strategy bots.

Usage:
    python scripts/train_vs_bots.py --total-timesteps 500_000 --bot-difficulty normal
"""

import argparse
import os

import gymnasium as gym
import numpy as np
from sb3_contrib import MaskablePPO

from powergrid_env import PowerGridSingleAgentEnv


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-players", type=int, default=4)
    parser.add_argument("--learner-seat", type=int, default=0)
    parser.add_argument("--bot-difficulty", default="normal", choices=["easy", "normal", "hard"])
    parser.add_argument("--total-timesteps", type=int, default=500_000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--device", default="auto")
    parser.add_argument("--run-dir", default="runs/vs_bots")
    args = parser.parse_args()

    os.makedirs(args.run_dir, exist_ok=True)

    env = PowerGridSingleAgentEnv(
        num_players=args.num_players,
        learner_seat=args.learner_seat,
        bot_difficulty=args.bot_difficulty,
        seed=args.seed,
        reward_shaping=True,
    )

    model = MaskablePPO(
        "MlpPolicy",
        env,
        verbose=1,
        seed=args.seed,
        device=args.device,
        tensorboard_log=os.path.join(args.run_dir, "tb"),
    )
    model.learn(total_timesteps=args.total_timesteps)
    model.save(os.path.join(args.run_dir, "final_model"))
    print(f"Saved to {args.run_dir}/final_model")
    env.close()


if __name__ == "__main__":
    main()
