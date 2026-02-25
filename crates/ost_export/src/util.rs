pub fn convert_to_pascal_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| word.chars().next().unwrap().to_uppercase().to_string() + &word[1..])
        .collect::<String>()
}
