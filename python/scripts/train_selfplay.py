"""
Self-play training: shared-policy MaskablePPO across all seats.

Usage:
    python scripts/train_selfplay.py --num-players 4 --total-timesteps 1_000_000
"""

import argparse
import os

import supersuit as ss
from sb3_contrib import MaskablePPO
from sb3_contrib.common.wrappers import ActionMasker

from powergrid_env import PowerGridAECEnv


def mask_fn(env):
    return env.infos[env.agent_selection].get("action_mask")


def make_env(num_players: int, seed: int):
    raw = PowerGridAECEnv(num_players=num_players, seed=seed, reward_shaping=False)
    # PettingZoo → Gymnasium (single vectorised learner, parameter sharing).
    vec = ss.pettingzoo_env_to_vec_env_v1(raw)
    vec = ss.concat_vec_envs_v1(vec, 1, base_class="stable_baselines3")
    return vec


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-players", type=int, default=4)
    parser.add_argument("--total-timesteps", type=int, default=1_000_000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--device", default="auto")
    parser.add_argument("--run-dir", default="runs/selfplay")
    args = parser.parse_args()

    os.makedirs(args.run_dir, exist_ok=True)

    vec_env = make_env(args.num_players, args.seed)

    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=1,
        seed=args.seed,
        device=args.device,
        tensorboard_log=os.path.join(args.run_dir, "tb"),
    )
    model.learn(total_timesteps=args.total_timesteps)
    model.save(os.path.join(args.run_dir, "final_model"))
    print(f"Saved to {args.run_dir}/final_model")
    vec_env.close()


if __name__ == "__main__":
    main()
