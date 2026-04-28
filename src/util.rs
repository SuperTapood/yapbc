pub fn pascal_to_snake(input: &str) -> String {
    let mut snake = String::new();

    for (i, ch) in input.char_indices() {
        if ch.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.extend(ch.to_lowercase());
        } else {
            snake.push(ch);
        }
    }

    snake
}

pub fn snake_to_pascal(input: &str) -> String {
    input
        .split('_')
        .filter(|word| !word.is_empty()) // Handles double underscores like "my__word"
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    // Uppercase the first char, lowercase the rest, and combine
                    first.to_uppercase().collect::<String>() + &chars.as_str()
                }
            }
        })
        .collect()
}

pub fn capitalize_first(s: &str) -> String {
    s.chars()
        .take(1)
        .flat_map(|f| f.to_uppercase())
        .chain(s.chars().skip(1))
        .collect()
}