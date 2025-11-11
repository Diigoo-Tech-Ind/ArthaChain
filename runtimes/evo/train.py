#!/usr/bin/env python3
"""Evolutionary Algorithm Training Script"""
import os
import neat
import requests

JOB_ID = os.environ.get("ARTHA_JOB_ID", "unknown")
GENERATIONS = int(os.environ.get("GENERATIONS", "30"))
POPULATION = int(os.environ.get("POPULATION", "50"))
PROOF_SERVICE_URL = os.environ.get("PROOF_SERVICE_URL", "http://localhost:8085")

def submit_evo_proof(generation: int, best_fitness: float, digest: str):
    try:
        requests.post(f"{PROOF_SERVICE_URL}/proof/submit",
            json={"job_id": JOB_ID, "proof_type": "EvolutionStep", "step": generation, "loss": -best_fitness}, timeout=10)
    except:
        pass

def eval_genome(genome, config):
    """Fitness function - XOR example"""
    net = neat.nn.FeedForwardNetwork.create(genome, config)
    inputs = [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)]
    outputs = [(0.0,), (1.0,), (1.0,), (0.0,)]
    error = 0.0
    for xi, xo in zip(inputs, outputs):
        output = net.activate(xi)
        error += (output[0] - xo[0]) ** 2
    return 4.0 - error  # Higher is better

def main():
    print(f"🚀 Evolutionary Training - Job: {JOB_ID}")
    
    # Load config
    config_path = os.path.join(os.path.dirname(__file__), "config.txt")
    if not os.path.exists(config_path):
        # Create basic config
        config = neat.Config(neat.DefaultGenome, neat.DefaultReproduction,
                            neat.DefaultSpeciesSet, neat.DefaultStagnation,
                            config_path)
    else:
        config = neat.Config(neat.DefaultGenome, neat.DefaultReproduction,
                            neat.DefaultSpeciesSet, neat.DefaultStagnation,
                            config_path)
    
    # Create population
    p = neat.Population(config)
    
    # Add reporters
    p.add_reporter(neat.StdOutReporter(True))
    
    # Run evolution
    best_genome = p.run(eval_genome, GENERATIONS)
    
    # Submit final proof
    best_fitness = eval_genome(best_genome, config)
    submit_evo_proof(GENERATIONS, best_fitness, f"0x{hash(str(GENERATIONS)):016x}")
    
    print(f"✅ Evolution complete! Best fitness: {best_fitness}")

if __name__ == "__main__":
    main()

