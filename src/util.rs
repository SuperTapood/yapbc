pub fn pascal_to_snake(input: &str) -> String {
    let mut snake = String::new();
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];
        if i > 0 && ch.is_uppercase() {
            let prev = chars[i - 1];
            // Only add underscore if transitioning from lowercase to uppercase
            // or if the acronym is ending (e.g., XMLParser -> xml_parser)
            if prev.is_lowercase() || (i + 1 < chars.len() && chars[i + 1].is_lowercase()) {
                snake.push('_');
            }
        }
        snake.extend(ch.to_lowercase());
    }
    // For your specific case 'k8sstack', if the input was 'K8SStack',
    // protoc often just lowers the whole thing if it doesn't detect clear word boundaries.
    snake
}

pub fn snake_to_pascal(input: &str) -> String {
    input
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
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
