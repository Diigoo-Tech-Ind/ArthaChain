#!/usr/bin/env python3
"""JAX Inference Script"""
import os
import jax
import jax.numpy as jnp
from flax import linen as nn

class SimpleModel(nn.Module):
    @nn.compact
    def __call__(self, x):
        x = nn.Dense(128)(x)
        x = nn.relu(x)
        return nn.Dense(10)(x)

def main():
    model = SimpleModel()
    key = jax.random.PRNGKey(0)
    params = model.init(key, jnp.ones((1, 784)))
    output = model.apply(params, jnp.ones((1, 784)))
    print(f"✅ Inference complete: {output}")

if __name__ == "__main__":
    main()

