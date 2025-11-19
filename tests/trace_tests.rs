use perchance_interpreter::{run_with_seed_and_trace, OperationType};

#[tokio::test]
async fn test_basic_trace() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n";
    let (output, trace) = run_with_seed_and_trace(template, 42, None).await.unwrap();

    // Should have output
    assert!(!output.is_empty());

    // Should have a root trace node
    assert_eq!(trace.operation, "Evaluate program");
    assert_eq!(trace.operation_type, Some(OperationType::Root));
    assert_eq!(trace.result, output);

    // Should have children (list evaluations)
    assert!(!trace.children.is_empty());

    println!("Output: {}", output);
    println!("Trace: {:#?}", trace);
}

#[tokio::test]
async fn test_trace_with_nested_lists() {
    let template = r#"
animal
    dog
    cat

color
    red
    blue

output
    I saw a [color] [animal].
"#;

    let (output, trace) = run_with_seed_and_trace(template, 123, None).await.unwrap();

    // Should have output
    assert!(output.contains("saw"));

    // Root should have children
    assert!(!trace.children.is_empty());

    // Find list select operations in trace
    fn count_list_selects(node: &perchance_interpreter::TraceNode) -> usize {
        let mut count = 0;
        if node.operation_type == Some(OperationType::ListSelect) {
            count += 1;
        }
        for child in &node.children {
            count += count_list_selects(child);
        }
        count
    }

    let list_select_count = count_list_selects(&trace);
    // Should have at least 2 list selects (color and animal, plus output)
    assert!(
        list_select_count >= 2,
        "Expected at least 2 list selects, got {}",
        list_select_count
    );

    println!("Output: {}", output);
    println!("List selects: {}", list_select_count);
}

#[tokio::test]
async fn test_trace_deterministic() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";

    let (output1, trace1) = run_with_seed_and_trace(template, 999, None).await.unwrap();
    let (output2, trace2) = run_with_seed_and_trace(template, 999, None).await.unwrap();

    // Same seed should produce same output and trace
    assert_eq!(output1, output2);
    assert_eq!(trace1.result, trace2.result);
    assert_eq!(trace1.children.len(), trace2.children.len());
}
