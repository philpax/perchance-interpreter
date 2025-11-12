use perchance_interpreter::{compile_template, evaluate, evaluate_with_seed};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Result type for evaluation operations
#[derive(Serialize, Deserialize)]
#[wasm_bindgen]
pub struct EvalResult {
    success: bool,
    output: String,
    error: Option<String>,
}

#[wasm_bindgen]
impl EvalResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn output(&self) -> String {
        self.output.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

/// Evaluate a Perchance template with a specific seed
#[wasm_bindgen]
pub fn evaluate_perchance(template: &str, seed: u64) -> EvalResult {
    match evaluate_with_seed(template, seed) {
        Ok(output) => EvalResult {
            success: true,
            output,
            error: None,
        },
        Err(e) => EvalResult {
            success: false,
            output: String::new(),
            error: Some(format!("{}", e)),
        },
    }
}

/// Evaluate a Perchance template with a random seed
#[wasm_bindgen]
pub fn evaluate_perchance_random(template: &str) -> EvalResult {
    match compile_template(template) {
        Ok(compiled) => {
            let mut rng = StdRng::from_entropy();
            match evaluate(&compiled, &mut rng) {
                Ok(output) => EvalResult {
                    success: true,
                    output,
                    error: None,
                },
                Err(e) => EvalResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("{}", e)),
                },
            }
        }
        Err(e) => EvalResult {
            success: false,
            output: String::new(),
            error: Some(format!("{}", e)),
        },
    }
}

/// Generate multiple samples from a template
#[wasm_bindgen]
pub fn evaluate_multiple(template: &str, count: u32, seed: Option<u64>) -> JsValue {
    let compiled = match compile_template(template) {
        Ok(c) => c,
        Err(e) => {
            let results: Vec<EvalResult> = vec![EvalResult {
                success: false,
                output: String::new(),
                error: Some(format!("{}", e)),
            }];
            return serde_wasm_bindgen::to_value(&results).unwrap();
        }
    };

    let mut results = Vec::new();
    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    for _ in 0..count {
        match evaluate(&compiled, &mut rng) {
            Ok(output) => results.push(EvalResult {
                success: true,
                output,
                error: None,
            }),
            Err(e) => results.push(EvalResult {
                success: false,
                output: String::new(),
                error: Some(format!("{}", e)),
            }),
        }
    }

    serde_wasm_bindgen::to_value(&results).unwrap()
}

/// Validate a template without evaluating it
#[wasm_bindgen]
pub fn validate_template(template: &str) -> EvalResult {
    match compile_template(template) {
        Ok(_) => EvalResult {
            success: true,
            output: String::from("Template is valid"),
            error: None,
        },
        Err(e) => EvalResult {
            success: false,
            output: String::new(),
            error: Some(format!("{}", e)),
        },
    }
}
