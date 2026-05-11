"""
Train a single-agent MaskablePPO policy vs Rust strategy bots.

Usage:
    python scripts/train_vs_bots.py --total-timesteps 500_000 --bot-difficulty normal
"""

import argparse
import os

from sb3_contrib import MaskablePPO
from stable_baselines3.common.callbacks import CheckpointCallback

from powergrid_env import PowerGridSingleAgentEnv


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-players", type=int, default=4)
    parser.add_argument("--learner-seat", type=int, default=0)
    parser.add_argument("--bot-difficulty", default="normal", choices=["easy", "normal", "hard"])
    parser.add_argument("--total-timesteps", type=int, default=500_000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--device", default="cpu",
                        help="PyTorch device. 'cpu' is usually fastest for the default "
                             "tiny 2×64 MLP; 'auto' picks GPU if available.")
    parser.add_argument("--run-dir", default="runs/vs_bots")
    parser.add_argument("--resume-from", default=None,
                        help="Path to a saved MaskablePPO .zip (without .zip suffix) "
                             "to continue training from. If unset, training starts fresh.")
    parser.add_argument("--save-freq", type=int, default=50_000,
                        help="Save an intermediate checkpoint every N env steps. 0 disables.")
    args = parser.parse_args()

    os.makedirs(args.run_dir, exist_ok=True)

    env = PowerGridSingleAgentEnv(
        num_players=args.num_players,
        learner_seat=args.learner_seat,
        bot_difficulty=args.bot_difficulty,
        seed=args.seed,
        reward_shaping=True,
    )

    if args.resume_from:
        model = MaskablePPO.load(args.resume_from, env=env, device=args.device)
        model.tensorboard_log = os.path.join(args.run_dir, "tb")
        print(f"Resumed from {args.resume_from} at {model.num_timesteps} timesteps")
    else:
        model = MaskablePPO(
            "MlpPolicy",
            env,
            verbose=1,
            seed=args.seed,
            device=args.device,
            tensorboard_log=os.path.join(args.run_dir, "tb"),
        )

    callbacks = []
    if args.save_freq > 0:
        callbacks.append(CheckpointCallback(
            save_freq=args.save_freq,
            save_path=args.run_dir,
            name_prefix="ckpt",
        ))

    model.learn(
        total_timesteps=args.total_timesteps,
        callback=callbacks or None,
        reset_num_timesteps=not bool(args.resume_from),
    )
    model.save(os.path.join(args.run_dir, "final_model"))
    print(f"Saved to {args.run_dir}/final_model")
    env.close()


if __name__ == "__main__":
    main()
