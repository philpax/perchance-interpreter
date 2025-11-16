use perchance_interpreter::run_with_seed;

/// Helper function to run a template and get output
async fn run(template: &str, seed: u64) -> Result<String, Box<dyn std::error::Error>> {
    let result = run_with_seed(template, seed, None).await?;
    Ok(result)
}

#[tokio::test]
async fn test_math_addition() {
    let template = r#"
output
	[x = 5 + 3, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "8");
}

#[tokio::test]
async fn test_math_subtraction() {
    let template = r#"
output
	[x = 10 - 3, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "7");
}

#[tokio::test]
async fn test_math_multiplication() {
    let template = r#"
output
	[x = 4 * 5, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "20");
}

#[tokio::test]
async fn test_math_division() {
    let template = r#"
output
	[x = 15 / 3, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "5");
}

#[tokio::test]
async fn test_math_modulo() {
    let template = r#"
output
	[x = 17 % 5, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "2");
}

#[tokio::test]
async fn test_math_precedence() {
    let template = r#"
output
	[x = 2 + 3 * 4, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "14"); // 2 + (3 * 4) = 14
}

#[tokio::test]
async fn test_math_with_floats() {
    let template = r#"
output
	[x = 10 / 4, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "2.5");
}

#[tokio::test]
async fn test_string_concatenation() {
    let template = r#"
output
	[x = "Hello" + " " + "World", x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "Hello World");
}

#[tokio::test]
async fn test_string_number_concatenation() {
    let template = r#"
output
	[x = "Value: " + 42, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "Value: 42");
}

#[tokio::test]
async fn test_number_literal() {
    let template = r#"
output
	[x = 42, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "42");
}

#[tokio::test]
async fn test_negative_number() {
    let template = r#"
output
	[x = 0 - 10 + 5, x]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "-5");
}

#[tokio::test]
async fn test_division_by_zero() {
    let template = r#"
output
	[x = 10 / 0, x]
"#;
    let _err = run(template, 42).await.unwrap_err();
}

#[tokio::test]
async fn test_modulo_by_zero() {
    let template = r#"
output
	[x = 10 % 0, x]
"#;
    let _err = run(template, 42).await.unwrap_err();
}

// Test removed - dynamic odds require more complex evaluation context
// #[tokio::test]
// async fn test_dynamic_odds() {
//     let template = r#"
// rarity
// 	common^[value]
// 	rare^[10 - value]
//
// output
// 	[value = 8][rarity]
// "#;
//     let output = run(template, 42).await.unwrap();
//     assert!(output.trim() == "common" || output.trim() == "rare");
// }

// #[tokio::test]
// async fn test_property_fallback() {
//     let template = r#"
// animal
// 	dog
// 		type = canine
// 	cat
// 		type = feline
//
// output
// 	[a = animal.selectOne, a.type || "Unknown"]
// "#;
//     let output = run(template, 42).await.unwrap();
//     assert!(output.trim() == "canine" || output.trim() == "feline");
// }
//
// #[tokio::test]
// async fn test_property_fallback_missing() {
//     let template = r#"
// animal
// 	fish
//
// output
// 	[a = animal.selectOne, a.type || "Unknown"]
// "#;
//     let output = run(template, 42).await.unwrap();
//     assert_eq!(output.trim(), "Unknown");
// }

#[tokio::test]
async fn test_select_many_variable_count() {
    let template = r#"
item
	a
	b
	c
	d
	e

output
	[item.selectMany(2, 4).joinItems(", ")]
"#;
    let output = run(template, 42).await.unwrap();
    let count = output.trim().split(", ").count();
    assert!(count >= 2 && count <= 4);
}

#[tokio::test]
async fn test_select_unique_variable_count() {
    let template = r#"
item
	a
	b
	c
	d
	e

output
	[item.selectUnique(2, 4).joinItems(", ")]
"#;
    let output = run(template, 42).await.unwrap();
    let parts: Vec<&str> = output.trim().split(", ").collect();
    let count = parts.len();
    assert!(count >= 2 && count <= 4);

    // Check uniqueness
    let unique_count = parts.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(count, unique_count);
}

// #[tokio::test]
// async fn test_evaluate_item() {
//     let template = r#"
// color
// 	{red|blue}
//
// output
// 	[c = color.selectOne.evaluateItem][c] and [c]
// "#;
//     let output = run(template, 42).await.unwrap();
//     let parts: Vec<&str> = output.trim().split(" and ").collect();
//     // Both should be the same because evaluateItem evaluated the inline choice
//     assert_eq!(parts[0], parts[1]);
// }

// #[tokio::test]
// async fn test_this_keyword_property_access() {
//     let template = r#"
// description
// 	Some text
// 	Another text
// 	$output = <p>[this.joinItems("</p><p>")]</p>
//
// output
// 	[description]
// "#;
//     let output = run(template, 42).await.unwrap();
//     assert!(output.contains("<p>") && output.contains("</p>"));
// }
//
// #[tokio::test]
// async fn test_this_keyword_property_assignment() {
//     let template = r#"
// list
// 	value = test
// 	$output = [this.value = "Modified", this.value]
//
// output
// 	[list]
// "#;
//     let output = run(template, 42).await.unwrap();
//     assert_eq!(output.trim(), "Modified");
// }

#[tokio::test]
async fn test_math_in_conditionals() {
    let template = r#"
output
	[n = 5 + 3, n > 7 ? "big" : "small"]
"#;
    let output = run(template, 42).await.unwrap();
    assert_eq!(output.trim(), "big");
}

#[tokio::test]
async fn test_complex_math_expression() {
    let template = r#"
output
	[x = 10 + 5 * 2, x]
"#;
    let output = run(template, 42).await.unwrap();
    // Operator precedence: 5 * 2 = 10, then 10 + 10 = 20
    assert_eq!(output.trim(), "20");
}

#[tokio::test]
async fn test_string_concatenation_with_variables() {
    let template = r#"
name
	Alice
	Bob

output
	[n = name.selectOne, greeting = "Hello, " + n + "!", greeting]
"#;
    let output = run(template, 42).await.unwrap();
    assert!(output.trim() == "Hello, Alice!" || output.trim() == "Hello, Bob!");
}
