/// CLI tool for Perchance interpreter
use perchance_interpreter::{compile, diagnostic, evaluate, parse, run_with_seed, EvaluateOptions};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  perchance <file> [seed]          Evaluate a template file with optional seed");
    eprintln!("  perchance -                       Read template from stdin");
    eprintln!("  perchance --help                  Show this help message");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  <file>      Path to Perchance template file");
    eprintln!("  [seed]      Optional seed for deterministic output (default: random)");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  perchance template.perchance      # Random output");
    eprintln!("  perchance template.perchance 42   # Deterministic output with seed 42");
    eprintln!("  cat template.perchance | perchance -   # Read from stdin");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    if args[1] == "--help" || args[1] == "-h" {
        print_usage();
        process::exit(0);
    }

    // Read template
    let template = if args[1] == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap_or_else(|e| {
            eprintln!("Error reading from stdin: {}", e);
            process::exit(1);
        });
        buffer
    } else {
        // Read from file
        fs::read_to_string(&args[1]).unwrap_or_else(|e| {
            eprintln!("Error reading file '{}': {}", args[1], e);
            process::exit(1);
        })
    };

    // Determine source name for diagnostics
    let source_name = if args[1] == "-" {
        "<stdin>"
    } else {
        &args[1]
    };

    // Parse seed if provided
    let result = if args.len() > 2 {
        let seed = args[2].parse::<u64>().unwrap_or_else(|e| {
            eprintln!("Error parsing seed '{}': {}", args[2], e);
            process::exit(1);
        });
        run_with_seed(&template, seed, None).await
    } else {
        // No seed provided, use random seed
        let program = parse(&template).unwrap_or_else(|e| {
            let diagnostic = diagnostic::report_parse_error(source_name, &template, &e);
            eprint!("{}", diagnostic);
            process::exit(1);
        });
        let compiled = compile(&program).unwrap_or_else(|e| {
            let diagnostic = diagnostic::report_compile_error(source_name, &template, &e);
            eprint!("{}", diagnostic);
            process::exit(1);
        });

        let rng = StdRng::from_entropy();
        let options = EvaluateOptions::new(rng);
        evaluate(&compiled, options).await.map_err(|e| e.into())
    };

    match result {
        Ok(output) => {
            println!("{}", output);
        }
        Err(e) => {
            let diagnostic = diagnostic::report_interpreter_error(source_name, &template, &e);
            eprint!("{}", diagnostic);
            process::exit(1);
        }
    }
}
