use perchance_interpreter::{compile_template, evaluate, evaluate_with_seed};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

/// Evaluate a Perchance template with a specific seed
/// Returns a Promise that resolves to a string
#[wasm_bindgen]
pub fn evaluate_perchance(template: String, seed: u64) -> js_sys::Promise {
    future_to_promise(async move {
        evaluate_with_seed(&template, seed)
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
        let compiled = compile_template(&template)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        let mut rng = StdRng::from_entropy();
        evaluate(&compiled, &mut rng)
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
        let compiled = compile_template(&template)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;

        let mut results = Vec::new();
        let mut rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        for _ in 0..count {
            let output = evaluate(&compiled, &mut rng)
                .await
                .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
            results.push(output);
        }

        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))
    })
}

/// Validate a template without evaluating it
#[wasm_bindgen]
pub fn validate_template(template: &str) -> Result<(), String> {
    compile_template(template)
        .map(|_| ())
        .map_err(|e| format!("{}", e))
}
