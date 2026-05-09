"""End-to-end random rollout tests."""
import json
import numpy as np
import pytest

from powergrid_env import PowerGridAECEnv, RandomPolicy


def play_game(num_players: int, seed: int) -> dict:
    """Play one complete game with random policy. Returns final state dict."""
    env = PowerGridAECEnv(num_players=num_players, seed=seed)
    env.reset()
    policy = RandomPolicy(rng=np.random.default_rng(seed))

    max_steps = 5000
    steps = 0
    for agent in env.agent_iter():
        obs, reward, terminated, truncated, info = env.last()
        if terminated or truncated:
            env.step(None)
        else:
            mask = info.get("action_mask", np.ones(env.action_space(agent).n, dtype=np.int8))
            action = policy.act(obs, mask)
            env.step(action)
        steps += 1
        if steps >= max_steps:
            break

    final_state = json.loads(env.game.state_json()) if env.game else {}
    env.close()
    return final_state, steps


@pytest.mark.parametrize("seed", range(10))
def test_2player_completes(seed):
    state, steps = play_game(2, seed)
    phase = state.get("phase", {})
    phase_key = list(phase.keys())[0] if isinstance(phase, dict) else phase
    assert phase_key == "game_over", f"seed={seed} phase={phase_key} steps={steps}"


@pytest.mark.parametrize("seed", range(5))
def test_4player_completes(seed):
    state, steps = play_game(4, seed)
    phase = state.get("phase", {})
    phase_key = list(phase.keys())[0] if isinstance(phase, dict) else phase
    assert phase_key == "game_over", f"seed={seed} phase={phase_key} steps={steps}"


def test_winner_exists():
    state, _ = play_game(3, seed=42)
    phase = state.get("phase", {})
    assert isinstance(phase, dict) and "game_over" in phase
    winner = phase["game_over"]["winner"]
    player_ids = [p["id"] for p in state.get("players", [])]
    assert winner in player_ids


def test_no_invalid_actions():
    """Ensure the action mask prevents all ValueError rejections for 50 random games."""
    errors = 0
    for seed in range(50):
        env = PowerGridAECEnv(num_players=2, seed=seed)
        env.reset()
        policy = RandomPolicy(rng=np.random.default_rng(seed))
        for agent in env.agent_iter():
            obs, reward, terminated, truncated, info = env.last()
            if terminated or truncated:
                env.step(None)
            else:
                mask = info.get("action_mask", np.zeros(env.action_space(agent).n, dtype=np.int8))
                if mask.sum() == 0:
                    errors += 1
                    env.step(0)
                    continue
                action = policy.act(obs, mask)
                if reward == -1.0 and terminated:
                    errors += 1
                env.step(action)
        env.close()
    assert errors == 0, f"{errors} invalid-action events across 50 games"
