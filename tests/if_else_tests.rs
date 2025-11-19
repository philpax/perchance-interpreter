use perchance_interpreter::run_with_seed;

/// Helper function to run a template and get output
async fn run(template: &str, seed: u64) -> Result<String, Box<dyn std::error::Error>> {
    let result = run_with_seed(template, seed, None).await?;
    Ok(result)
}

#[tokio::test]
async fn test_long_form_if_else_simple() {
    let template = r#"
output
	[n = 3, if (n < 5) {"small"} else {"big"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "small");
}

#[tokio::test]
async fn test_long_form_if_else_false() {
    let template = r#"
output
	[n = 7, if (n < 5) {"small"} else {"big"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "big");
}

#[tokio::test]
async fn test_long_form_if_else_with_list_references() {
    let template = r#"
small_things
	ant
	mouse

big_things
	elephant
	whale

output
	[n = 2, if (n < 5) {small_things} else {big_things}]
"#;
    let output = run(template, 42).await.unwrap();
    assert!(output.trim() == "ant" || output.trim() == "mouse");
}

#[tokio::test]
async fn test_long_form_else_if_chain() {
    let template = r#"
output
	[n = 3, if (n < 2) {"tiny"} else if (n < 5) {"small"} else if (n < 8) {"medium"} else {"large"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "small");
}

#[tokio::test]
async fn test_long_form_else_if_last_branch() {
    let template = r#"
output
	[n = 10, if (n < 2) {"tiny"} else if (n < 5) {"small"} else if (n < 8) {"medium"} else {"large"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "large");
}

#[tokio::test]
async fn test_long_form_else_if_middle_branch() {
    let template = r#"
output
	[n = 6, if (n < 2) {"tiny"} else if (n < 5) {"small"} else if (n < 8) {"medium"} else {"large"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "medium");
}

#[tokio::test]
async fn test_long_form_if_without_else() {
    let template = r#"
output
	[n = 3, result = if (n < 5) {"matched"}, result]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "matched");
}

#[tokio::test]
async fn test_long_form_if_without_else_no_match() {
    let template = r#"
output
	[n = 7, result = if (n < 5) {"matched"}, "done"]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "done");
}

#[tokio::test]
async fn test_long_form_with_complex_conditions() {
    let template = r#"
output
	[x = 3, y = 4, if (x < 5 && y > 2) {"yes"} else {"no"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "yes");
}

#[tokio::test]
async fn test_long_form_with_string_comparison() {
    let template = r#"
name
	Alice
	Bob

output
	[n = name.selectOne, if (n == "Alice") {"Hello Alice!"} else {"Hello stranger!"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert!(output.trim() == "Hello Alice!" || output.trim() == "Hello stranger!");
}

#[tokio::test]
async fn test_long_form_nested() {
    let template = r#"
output
	[x = 3, if (x < 5) {if (x < 3) {"very small"} else {"small"}} else {"big"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "small");
}

#[tokio::test]
async fn test_long_form_with_expressions_in_body() {
    let template = r#"
output
	[n = 3, if (n < 5) {n + 10} else {n + 100}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "13");
}

#[tokio::test]
async fn test_comparison_with_ternary() {
    // Both should give same result
    let template1 = r#"
output
	[n = 3, if (n < 5) {"small"} else {"big"}]
"#;
    let template2 = r#"
output
	[n = 3, n < 5 ? "small" : "big"]
"#;
    let output1 = run(template1, 42).await.unwrap();
    let output2 = run(template2, 42).await.unwrap();
    assert_eq!(output1.trim(), output2.trim());
    assert_eq!(output1.trim(), "small");
}
