/// Symbolic AI / Theory of Mind Service
/// Provides rule-based reasoning and mental state modeling

use axum::{
    extract::{Path, State, Json, Query},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct SymbolicReasoningRequest {
    pub rules: Vec<Rule>,
    pub facts: Vec<Fact>,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub head: String,  // Conclusion
    pub body: Vec<String>,  // Premises
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub predicate: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SymbolicReasoningResponse {
    pub result: bool,
    pub proof: Vec<String>,
    pub bindings: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct TheoryOfMindRequest {
    pub agent_id: String,
    pub observed_actions: Vec<Action>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub target: String,
    pub parameters: HashMap<String, String>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct TheoryOfMindResponse {
    pub beliefs: Vec<String>,
    pub desires: Vec<String>,
    pub intentions: Vec<String>,
    pub predicted_next_action: Option<String>,
    pub confidence: f64,
}

pub struct AppState {
    rules_engine: Arc<RwLock<RulesEngine>>,
    mental_models: Arc<RwLock<HashMap<String, MentalModel>>>,
}

struct RulesEngine {
    rules: Vec<Rule>,
    facts: Vec<Fact>,
}

struct MentalModel {
    agent_id: String,
    beliefs: Vec<String>,
    desires: Vec<String>,
    intentions: Vec<String>,
    history: Vec<Action>,
}

async fn reason_symbolically(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SymbolicReasoningRequest>,
) -> Result<Json<SymbolicReasoningResponse>, StatusCode> {
    let mut engine = RulesEngine {
        rules: req.rules.clone(),
        facts: req.facts.clone(),
    };
    
    // Forward chaining inference
    let (result, proof, bindings) = forward_chain(&mut engine, &req.query);
    
    Ok(Json(SymbolicReasoningResponse {
        result,
        proof,
        bindings,
    }))
}

fn forward_chain(
    engine: &mut RulesEngine,
    query: &str,
) -> (bool, Vec<String>, HashMap<String, String>) {
    let mut proof = Vec::new();
    let mut bindings = HashMap::new();
    let mut working_memory = engine.facts.clone();
    let mut changed = true;
    
    while changed {
        changed = false;
        
        for rule in &engine.rules {
            if can_apply_rule(rule, &working_memory) {
                let new_fact = Fact {
                    predicate: rule.head.clone(),
                    args: vec![],
                };
                
                if !working_memory.contains(&new_fact) {
                    working_memory.push(new_fact.clone());
                    proof.push(format!("Applied rule: {} :- {}", rule.head, rule.body.join(", ")));
                    changed = true;
                }
            }
        }
    }
    
    // Check if query is satisfied
    let result = working_memory.iter().any(|f| f.predicate == query);
    
    (result, proof, bindings)
}

fn can_apply_rule(rule: &Rule, facts: &[Fact]) -> bool {
    rule.body.iter().all(|premise| {
        facts.iter().any(|f| f.predicate == *premise)
    })
}

async fn theory_of_mind(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TheoryOfMindRequest>,
) -> Result<Json<TheoryOfMindResponse>, StatusCode> {
    let mut models = state.mental_models.write().await;
    
    // Get or create mental model
    let model = models.entry(req.agent_id.clone())
        .or_insert_with(|| MentalModel {
            agent_id: req.agent_id.clone(),
            beliefs: Vec::new(),
            desires: Vec::new(),
            intentions: Vec::new(),
            history: Vec::new(),
        });
    
    // Update history
    model.history.extend(req.observed_actions);
    
    // Infer beliefs from observed actions
    let beliefs = infer_beliefs(&model.history, &req.context);
    
    // Infer desires (goals)
    let desires = infer_desires(&model.history);
    
    // Infer intentions (planned actions)
    let intentions = infer_intentions(&beliefs, &desires);
    
    // Predict next action
    let predicted_action = predict_next_action(&beliefs, &desires, &intentions);
    
    // Update model
    model.beliefs = beliefs.clone();
    model.desires = desires.clone();
    model.intentions = intentions.clone();
    
    Ok(Json(TheoryOfMindResponse {
        beliefs,
        desires,
        intentions,
        predicted_next_action: predicted_action,
        confidence: 0.75,  // Confidence in predictions
    }))
}

fn infer_beliefs(history: &[Action], context: &HashMap<String, String>) -> Vec<String> {
    let mut beliefs = Vec::new();
    
    // Analyze actions to infer beliefs
    for action in history {
        match action.action_type.as_str() {
            "search" => beliefs.push("agent_wants_information".to_string()),
            "storage_put" => beliefs.push("agent_needs_persistence".to_string()),
            "transaction" => beliefs.push("agent_manages_resources".to_string()),
            _ => {}
        }
    }
    
    beliefs
}

fn infer_desires(history: &[Action]) -> Vec<String> {
    let mut desires = Vec::new();
    
    // Infer goals from action patterns
    let action_types: Vec<&str> = history.iter().map(|a| a.action_type.as_str()).collect();
    
    if action_types.contains(&"search") {
        desires.push("find_information".to_string());
    }
    if action_types.contains(&"storage_put") {
        desires.push("save_data".to_string());
    }
    if action_types.contains(&"transaction") {
        desires.push("manage_blockchain_resources".to_string());
    }
    
    desires
}

fn infer_intentions(beliefs: &[String], desires: &[String]) -> Vec<String> {
    let mut intentions = Vec::new();
    
    // Plan actions to achieve desires given beliefs
    for desire in desires {
        match desire.as_str() {
            "find_information" => {
                if !beliefs.contains(&"agent_has_information".to_string()) {
                    intentions.push("perform_search".to_string());
                }
            }
            "save_data" => {
                intentions.push("perform_storage".to_string());
            }
            _ => {}
        }
    }
    
    intentions
}

fn predict_next_action(
    beliefs: &[String],
    desires: &[String],
    intentions: &[String],
) -> Option<String> {
    // Predict based on current mental state
    if !intentions.is_empty() {
        Some(intentions[0].clone())
    } else if !desires.is_empty() {
        Some(format!("pursue_{}", desires[0]))
    } else {
        None
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        rules_engine: Arc::new(RwLock::new(RulesEngine {
            rules: Vec::new(),
            facts: Vec::new(),
        })),
        mental_models: Arc::new(RwLock::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/symbolic/reason", post(reason_symbolically))
        .route("/theory-of-mind/infer", post(theory_of_mind))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    println!("🧠 Symbolic AI / Theory of Mind Service starting on :8091");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8091").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

