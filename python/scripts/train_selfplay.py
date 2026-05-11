"""
Self-play training: shared-policy MaskablePPO across all seats.

Usage:
    python scripts/train_selfplay.py --num-players 4 --total-timesteps 1_000_000
    python scripts/train_selfplay.py --num-envs 8 --total-timesteps 5_000_000

Performance notes:
  - Each env step now calls Rust directly (no JSON serialisation) — ~14× faster
    raw env throughput vs the old JSON-bridge + PettingZoo wrapper chain.
  - All rollout transitions are real game steps (no black_death padding waste).
  - SubprocVecEnv is not used: each Rust step is so fast (~200 µs) that IPC
    overhead dominates. DummyVecEnv (sequential, in-process) is faster for this
    workload. A few envs (4–8) gives the best balance between env throughput and
    policy-forward amortisation.
"""

import argparse
import os

from sb3_contrib import MaskablePPO
from stable_baselines3.common.callbacks import CheckpointCallback
from stable_baselines3.common.vec_env import DummyVecEnv

from powergrid_env import PowerGridSelfPlayEnv


def make_env(num_players: int, seed: int):
    def _init():
        return PowerGridSelfPlayEnv(num_players=num_players, seed=seed)
    return _init


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--num-players", type=int, default=4)
    parser.add_argument("--num-envs", type=int, default=8,
                        help="Number of parallel envs (DummyVecEnv).")
    parser.add_argument("--total-timesteps", type=int, default=1_000_000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--device", default="cpu",
                        help="PyTorch device. 'cpu' is usually fastest for the default "
                             "tiny 2×64 MLP; 'auto' picks GPU if available.")
    parser.add_argument("--run-dir", default="runs/selfplay")
    parser.add_argument("--resume-from", default=None,
                        help="Path to a saved MaskablePPO .zip (without .zip suffix) "
                             "to continue training from. If unset, training starts fresh.")
    parser.add_argument("--save-freq", type=int, default=50_000,
                        help="Save an intermediate checkpoint every N vec-env steps. "
                             "0 disables.")
    args = parser.parse_args()

    os.makedirs(args.run_dir, exist_ok=True)

    env_fns = [make_env(args.num_players, args.seed + i) for i in range(args.num_envs)]
    vec_env = DummyVecEnv(env_fns)

    # n_epochs/batch_size are the dominant cost on CPU.
    # Default PPO (n_epochs=10, batch=64) does 1280 mini-batch updates per
    # rollout; these settings do 64 (8192/512 * 4 epochs), giving ~3s/iter
    # instead of ~18s/iter with no significant quality loss in practice.
    if args.resume_from:
        model = MaskablePPO.load(args.resume_from, env=vec_env, device=args.device)
        print(f"Resumed from {args.resume_from} at {model.num_timesteps} timesteps")
    else:
        model = MaskablePPO(
            "MlpPolicy",
            vec_env,
            verbose=1,
            seed=args.seed,
            device=args.device,
            n_steps=512,
            batch_size=512,
            n_epochs=4,
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
    vec_env.close()


if __name__ == "__main__":
    main()
