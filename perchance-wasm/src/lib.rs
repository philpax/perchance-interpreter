use perchance_interpreter::{
    compile, diagnostic, evaluate, parse, run_with_seed, run_with_seed_and_trace,
    EvaluateOptions, InterpreterError, TraceResult,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

/// Helper function to format errors with ariadne diagnostics
fn format_error(template: &str, error: &InterpreterError) -> String {
    diagnostic::report_interpreter_error("<input>", template, error)
}

/// Evaluate a Perchance template with a specific seed
/// Returns a Promise that resolves to a string
#[wasm_bindgen]
pub fn evaluate_perchance(template: String, seed: u64) -> js_sys::Promise {
    future_to_promise(async move {
        let template_clone = template.clone();
        run_with_seed(&template, seed, None)
            .await
            .map(|s| JsValue::from_str(&s))
            .map_err(|e| JsValue::from_str(&format_error(&template_clone, &e)))
    })
}

/// Evaluate a Perchance template with a random seed
/// Returns a Promise that resolves to a string
#[wasm_bindgen]
pub fn evaluate_perchance_random(template: String) -> js_sys::Promise {
    future_to_promise(async move {
        let template_clone = template.clone();
        let program = parse(&template).map_err(|e| {
            JsValue::from_str(&diagnostic::report_parse_error("<input>", &template, &e))
        })?;
        let compiled = compile(&program).map_err(|e| {
            JsValue::from_str(&diagnostic::report_compile_error("<input>", &template, &e))
        })?;
        let rng = StdRng::from_entropy();
        let options = EvaluateOptions::new(rng);
        evaluate(&compiled, options)
            .await
            .map(|s| JsValue::from_str(&s))
            .map_err(|e| {
                JsValue::from_str(&diagnostic::report_eval_error("<input>", &template_clone, &e))
            })
    })
}

/// Generate multiple samples from a template
/// Returns a Promise that resolves to a JS array of strings
#[wasm_bindgen]
pub fn evaluate_multiple(template: String, count: u32, seed: Option<u64>) -> js_sys::Promise {
    future_to_promise(async move {
        let mut results = Vec::new();
        let template_clone = template.clone();

        // If seed is provided, use sequential seeds for deterministic but varied output
        if let Some(base_seed) = seed {
            for i in 0..count {
                let output = run_with_seed(&template, base_seed.wrapping_add(i as u64), None)
                    .await
                    .map_err(|e| JsValue::from_str(&format_error(&template, &e)))?;
                results.push(output);
            }
        } else {
            // No seed: parse/compile once, then evaluate multiple times
            let program = parse(&template).map_err(|e| {
                JsValue::from_str(&diagnostic::report_parse_error("<input>", &template, &e))
            })?;
            let compiled = compile(&program).map_err(|e| {
                JsValue::from_str(&diagnostic::report_compile_error("<input>", &template, &e))
            })?;

            for _ in 0..count {
                let rng = StdRng::from_entropy();
                let options = EvaluateOptions::new(rng);
                let output = evaluate(&compiled, options)
                    .await
                    .map_err(|e| {
                        JsValue::from_str(&diagnostic::report_eval_error("<input>", &template_clone, &e))
                    })?;
                results.push(output);
            }
        }

        serde_wasm_bindgen::to_value(&results).map_err(|e| JsValue::from_str(&format!("{}", e)))
    })
}

/// Validate a template without evaluating it
#[wasm_bindgen]
pub fn validate_template(template: &str) -> Result<(), String> {
    let program = parse(template).map_err(|e| {
        diagnostic::report_parse_error("<input>", template, &e)
    })?;
    compile(&program).map(|_| ()).map_err(|e| {
        diagnostic::report_compile_error("<input>", template, &e)
    })
}

/// Get list of all available builtin generators for autocomplete
/// Returns a JS array of generator names
#[wasm_bindgen]
pub fn get_available_generators() -> JsValue {
    #[cfg(feature = "builtin-generators")]
    {
        use perchance_interpreter::loader::{BuiltinGeneratorsLoader, GeneratorLoader};
        let loader = BuiltinGeneratorsLoader::new();
        let names = loader.list_available();
        serde_wasm_bindgen::to_value(&names).unwrap_or(JsValue::NULL)
    }
    #[cfg(not(feature = "builtin-generators"))]
    {
        // Return empty array if builtin generators not available
        let empty: Vec<String> = Vec::new();
        serde_wasm_bindgen::to_value(&empty).unwrap_or(JsValue::NULL)
    }
}

/// Evaluate a Perchance template with a specific seed and return both output and trace
/// Returns a Promise that resolves to a JS object with { output: string, trace: TraceNode }
#[wasm_bindgen]
pub fn evaluate_perchance_with_trace(template: String, seed: u64) -> js_sys::Promise {
    future_to_promise(async move {
        let template_clone = template.clone();
        let (output, trace) = run_with_seed_and_trace(&template, seed, None)
            .await
            .map_err(|e| JsValue::from_str(&format_error(&template_clone, &e)))?;

        let result = TraceResult::new(output, trace);
        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    })
}
