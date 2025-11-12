use perchance_interpreter::{compile_template, evaluate, evaluate_with_seed};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;

/// Evaluate a Perchance template with a specific seed
#[wasm_bindgen]
pub fn evaluate_perchance(template: &str, seed: u64) -> Result<String, String> {
    evaluate_with_seed(template, seed).map_err(|e| format!("{}", e))
}

/// Evaluate a Perchance template with a random seed
#[wasm_bindgen]
pub fn evaluate_perchance_random(template: &str) -> Result<String, String> {
    let compiled = compile_template(template).map_err(|e| format!("{}", e))?;
    let mut rng = StdRng::from_entropy();
    evaluate(&compiled, &mut rng).map_err(|e| format!("{}", e))
}

/// Generate multiple samples from a template
/// Returns a JS array of strings on success, or throws an error if template is invalid
#[wasm_bindgen]
pub fn evaluate_multiple(template: &str, count: u32, seed: Option<u64>) -> Result<JsValue, String> {
    let compiled = compile_template(template).map_err(|e| format!("{}", e))?;

    let mut results = Vec::new();
    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    for _ in 0..count {
        let output = evaluate(&compiled, &mut rng).map_err(|e| format!("{}", e))?;
        results.push(output);
    }

    serde_wasm_bindgen::to_value(&results).map_err(|e| format!("{}", e))
}

/// Validate a template without evaluating it
#[wasm_bindgen]
pub fn validate_template(template: &str) -> Result<(), String> {
    compile_template(template)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}
