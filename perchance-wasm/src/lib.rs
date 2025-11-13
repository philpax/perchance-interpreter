use perchance_interpreter::{compile, evaluate, parse, run_with_seed, EvaluateOptions};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

/// Evaluate a Perchance template with a specific seed
/// Returns a Promise that resolves to a string
#[wasm_bindgen]
pub fn evaluate_perchance(template: String, seed: u64) -> js_sys::Promise {
    future_to_promise(async move {
        run_with_seed(&template, seed, None)
            .await
            .map(|s| JsValue::from_str(&s))
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    })
}

/// Evaluate a Perchance template with a random seed
/// Returns a Promise that resolves to a string
#[wasm_bindgen]
pub fn evaluate_perchance_random(template: String) -> js_sys::Promise {
    future_to_promise(async move {
        let program = parse(&template).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        let compiled = compile(&program).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        let rng = StdRng::from_entropy();
        let options = EvaluateOptions::new(rng);
        evaluate(&compiled, options)
            .await
            .map(|s| JsValue::from_str(&s))
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    })
}

/// Generate multiple samples from a template
/// Returns a Promise that resolves to a JS array of strings
#[wasm_bindgen]
pub fn evaluate_multiple(template: String, count: u32, seed: Option<u64>) -> js_sys::Promise {
    future_to_promise(async move {
        let mut results = Vec::new();

        // If seed is provided, use sequential seeds for deterministic but varied output
        if let Some(base_seed) = seed {
            for i in 0..count {
                let output = run_with_seed(&template, base_seed.wrapping_add(i as u64), None)
                    .await
                    .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
                results.push(output);
            }
        } else {
            // No seed: parse/compile once, then evaluate multiple times
            let program = parse(&template).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
            let compiled = compile(&program).map_err(|e| JsValue::from_str(&format!("{}", e)))?;

            for _ in 0..count {
                let rng = StdRng::from_entropy();
                let options = EvaluateOptions::new(rng);
                let output = evaluate(&compiled, options)
                    .await
                    .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
                results.push(output);
            }
        }

        serde_wasm_bindgen::to_value(&results).map_err(|e| JsValue::from_str(&format!("{}", e)))
    })
}

/// Validate a template without evaluating it
#[wasm_bindgen]
pub fn validate_template(template: &str) -> Result<(), String> {
    let program = parse(template).map_err(|e| format!("{}", e))?;
    compile(&program).map(|_| ()).map_err(|e| format!("{}", e))
}
