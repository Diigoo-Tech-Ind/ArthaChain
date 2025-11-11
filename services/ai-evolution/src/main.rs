/// AI Evolution Service - Evolutionary Algorithm Coordinator
/// Handles NEAT, genetic algorithms, and evolutionary search

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionJob {
    pub evo_id: String,
    pub search_space_cid: String,
    pub population: u32,
    pub generations: u32,
    pub current_generation: u32,
    pub status: EvoStatus,
    pub best_fitness: f64,
    pub best_genome_cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvoStatus {
    Queued,
    Evolving,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub genome_id: String,
    pub genes: Vec<Gene>,
    pub fitness: f64,
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub from_node: u32,
    pub to_node: u32,
    pub weight: f64,
    pub enabled: bool,
    pub innovation: u32,
}

pub struct AppState {
    evo_jobs: Arc<RwLock<HashMap<String, EvolutionJob>>>,
    populations: Arc<RwLock<HashMap<String, Vec<Genome>>>>, // evo_id -> population
}

// NEAT algorithm implementation
fn mutate_genome(genome: &mut Genome, mutation_rate: f64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    if rng.gen::<f64>() < mutation_rate {
        // Weight mutation
        for gene in &mut genome.genes {
            if rng.gen::<f64>() < 0.8 {
                gene.weight += rng.gen::<f64>() * 0.1 - 0.05;
            }
        }
    }
    
    if rng.gen::<f64>() < mutation_rate * 0.1 {
        // Add connection
        // Simplified: add random connection
    }
    
    if rng.gen::<f64>() < mutation_rate * 0.03 {
        // Add node
        // Simplified: split a connection
    }
}

fn crossover(parent1: &Genome, parent2: &Genome) -> Genome {
    // Crossover two genomes
    let mut child = Genome {
        genome_id: uuid::Uuid::new_v4().to_string(),
        genes: Vec::new(),
        fitness: 0.0,
        generation: parent1.generation.max(parent2.generation) + 1,
    };
    
    // Merge genes from both parents
    // Simplified: take genes from fitter parent
    if parent1.fitness >= parent2.fitness {
        child.genes = parent1.genes.clone();
    } else {
        child.genes = parent2.genes.clone();
    }
    
    child
}

fn evolve_population(population: &mut Vec<Genome>, mutation_rate: f64) {
    // Sort by fitness
    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
    
    // Keep top 50%
    let elite_size = population.len() / 2;
    let mut new_population = population[..elite_size].to_vec();
    
    // Crossover and mutate to fill remaining
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    while new_population.len() < population.len() {
        let p1 = &population[rng.gen_range(0..elite_size)];
        let p2 = &population[rng.gen_range(0..elite_size)];
        let mut child = crossover(p1, p2);
        mutate_genome(&mut child, mutation_rate);
        new_population.push(child);
    }
    
    *population = new_population;
}

#[derive(Debug, Deserialize)]
pub struct StartEvolutionRequest {
    pub search_space_cid: String,
    pub population: u32,
    pub generations: u32,
    pub budget: u64,
}

#[derive(Debug, Serialize)]
pub struct StartEvolutionResponse {
    pub evo_id: String,
    pub status: String,
}

async fn start_evolution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartEvolutionRequest>,
) -> Result<Json<StartEvolutionResponse>, StatusCode> {
    let evo_id = format!("evo-{}", uuid::Uuid::new_v4());
    
    let evo_job = EvolutionJob {
        evo_id: evo_id.clone(),
        search_space_cid: req.search_space_cid,
        population: req.population,
        generations: req.generations,
        current_generation: 0,
        status: EvoStatus::Queued,
        best_fitness: 0.0,
        best_genome_cid: None,
    };

    state.evo_jobs.write().await.insert(evo_id.clone(), evo_job);

    // Start evolutionary search
    // TODO: Launch evo-runtime containers

    Ok(Json(StartEvolutionResponse {
        evo_id,
        status: "queued".to_string(),
    }))
}

async fn get_evo_status(
    State(state): State<Arc<AppState>>,
    Path(evo_id): Path<String>,
) -> Result<Json<EvolutionJob>, StatusCode> {
    let jobs = state.evo_jobs.read().await;
    let job = jobs.get(&evo_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(job.clone()))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        evo_jobs: Arc::new(RwLock::new(HashMap::new())),
        populations: Arc::new(RwLock::new(HashMap::new())),
    });

async fn get_evo_population(
    State(state): State<Arc<AppState>>,
    Path(evo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let populations = state.populations.read().await;
    let jobs = state.evo_jobs.read().await;
    let job = jobs.get(&evo_id).ok_or(StatusCode::NOT_FOUND)?;
    
    if let Some(pop) = populations.get(&evo_id) {
        Ok(Json(serde_json::json!({
            "evo_id": evo_id,
            "generation": job.current_generation,
            "population_size": pop.len(),
            "best_fitness": job.best_fitness,
            "genomes": pop.iter().map(|g| serde_json::json!({
                "genome_id": g.genome_id,
                "fitness": g.fitness,
            })).collect::<Vec<_>>(),
        })))
    } else {
        Ok(Json(serde_json::json!({
            "evo_id": evo_id,
            "generation": 0,
            "population_size": 0,
        })))
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        evo_jobs: Arc::new(RwLock::new(HashMap::new())),
        populations: Arc::new(RwLock::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/evolution/start", post(start_evolution))
        .route("/evolution/:id/status", get(get_evo_status))
        .route("/evolution/:id/population", get(get_evo_population))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🚀 AI Evolution Service starting on :8088");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8088").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

