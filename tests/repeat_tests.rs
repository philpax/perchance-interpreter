use perchance_interpreter::run_with_seed;

/// Helper function to run a template and get output
async fn run(template: &str, seed: u64) -> Result<String, Box<dyn std::error::Error>> {
    let result = run_with_seed(template, seed, None).await?;
    Ok(result)
}

#[tokio::test]
async fn test_repeat_simple() {
    let template = r#"
output
	[repeat(5) {"x"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "xxxxx");
}

#[tokio::test]
async fn test_repeat_with_list() {
    let template = r#"
char
	a
	b
	c

output
	[repeat(3) {char}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim().len(), 3); // Should have 3 characters
}

#[tokio::test]
async fn test_repeat_zero_times() {
    let template = r#"
output
	[repeat(0) {"x"}]empty
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "empty");
}

#[tokio::test]
async fn test_repeat_with_variable_count() {
    let template = r#"
output
	[n = 4, repeat(n) {"*"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "****");
}

#[tokio::test]
async fn test_repeat_with_expression_count() {
    let template = r#"
output
	[repeat(2 + 3) {"!"}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "!!!!!");
}

#[tokio::test]
async fn test_repeat_with_number_range() {
    let template = r#"
num
	{1-5}

output
	[repeat(3) {num}]
"#;
    let output = run(template, 42).await.unwrap();
    // Should generate 3 single-digit numbers
    assert!(!output.trim().is_empty() && output.trim().len() <= 3);
}

#[tokio::test]
async fn test_repeat_with_inline_list() {
    let template = r#"
choice
	{a|b}

output
	[repeat(4) {choice}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim().len(), 4); // Should have 4 characters (all a or b)
    assert!(output.trim().chars().all(|c| c == 'a' || c == 'b'));
}

#[tokio::test]
async fn test_repeat_with_text() {
    let template = r#"
output
	[repeat(3) {"Hello "}]World
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "Hello Hello Hello World");
}

#[tokio::test]
async fn test_repeat_nested() {
    let template = r#"
output
	[repeat(2) {repeat(3) {"x"}}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "xxxxxx"); // 2 * 3 = 6
}

#[tokio::test]
async fn test_repeat_with_different_counts() {
    let template1 = r#"
output
	[repeat(2) {"*"}]
"#;
    let template2 = r#"
output
	[repeat(5) {"*"}]
"#;
    let output1 = run(template1, 42).await.unwrap();
    let output2 = run(template2, 42).await.unwrap();

    assert_eq!(output1.trim(), "**");
    assert_eq!(output2.trim(), "*****");
}

#[tokio::test]
async fn test_repeat_comparison_with_selectmany() {
    // These should produce similar results (though not identical due to RNG)
    let template1 = r#"
char
	x

output
	[repeat(5) {char}]
"#;
    let template2 = r#"
char
	x

output
	[char.selectMany(5).joinItems("")]
"#;
    let output1 = run(template1, 42).await.unwrap();
    let output2 = run(template2, 42).await.unwrap();

    // Both should produce "xxxxx"
    assert_eq!(output1.trim(), "xxxxx");
    assert_eq!(output2.trim(), "xxxxx");
}

#[tokio::test]
async fn test_repeat_with_math() {
    let template = r#"
output
	[i = 0, repeat(5) {i = i + 1, i}]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "12345");
}
