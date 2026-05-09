"""
Self-play training: shared-policy MaskablePPO across all seats.

Usage:
    python scripts/train_selfplay.py --num-players 4 --total-timesteps 1_000_000
"""

import argparse
import os

import numpy as np
import supersuit as ss
from pettingzoo.utils.conversions import turn_based_aec_to_parallel
from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import VecEnvWrapper
from supersuit.vector.markov_vector_wrapper import MarkovVectorEnv

from powergrid_env import PowerGridAECEnv


class ActionMaskVecEnv(VecEnvWrapper):
    """Bridges supersuit's vec env to MaskablePPO's action_masks() protocol.

    MaskablePPO discovers masking via has_attr("action_masks") and retrieves
    masks via env_method("action_masks"). Supersuit's ConcatVecEnv doesn't
    implement either; this wrapper intercepts infos from each step/reset and
    exposes the masks through the expected interface.
    """

    def __init__(self, venv):
        super().__init__(venv)
        self._masks = np.ones((venv.num_envs, self.action_space.n), dtype=np.int8)

    def reset(self):
        obs = self.venv.reset()
        infos = getattr(self.venv, "reset_infos", None) or []
        self._update_masks(infos)
        return obs

    def step_wait(self):
        obs, rewards, dones, infos = self.venv.step_wait()
        self._update_masks(infos)
        return obs, rewards, dones, infos

    def _update_masks(self, infos):
        for i, info in enumerate(infos):
            mask = info.get("action_mask") if isinstance(info, dict) else None
            if mask is not None:
                self._masks[i] = mask

    def action_masks(self):
        return self._masks.copy()

    def has_attr(self, attr_name):
        if attr_name == "action_masks":
            return True
        try:
            return self.venv.has_attr(attr_name)
        except AttributeError:
            return False

    def env_method(self, method_name, *method_args, indices=None, **method_kwargs):
        if method_name == "action_masks":
            masks = self._masks
            if indices is not None:
                return [masks[i] for i in indices]
            return list(masks)
        try:
            return self.venv.env_method(
                method_name, *method_args, indices=indices, **method_kwargs
            )
        except AttributeError:
            raise AttributeError(f"env_method: {method_name} not supported")


def make_env(num_players: int, seed: int):
    raw = PowerGridAECEnv(num_players=num_players, seed=seed, reward_shaping=False)
    parallel = turn_based_aec_to_parallel(raw)
    # black_death=True feeds zero obs/rewards for terminated agents so the
    # vectorised env can keep running until all agents are done.
    vec = MarkovVectorEnv(parallel, black_death=True)
    vec = ss.concat_vec_envs_v1(vec, 1, base_class="stable_baselines3")
    vec = ActionMaskVecEnv(vec)
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
        device=args.device,
    )
    model.learn(total_timesteps=args.total_timesteps)
    model.save(os.path.join(args.run_dir, "final_model"))
    print(f"Saved to {args.run_dir}/final_model")
    vec_env.close()


if __name__ == "__main__":
    main()
