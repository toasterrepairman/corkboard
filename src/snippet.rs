use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub title: String,
    pub language: String,
    pub code: String,
}

impl Snippet {
    pub fn new(title: String, language: String, code: String) -> Self {
        Self {
            title,
            language,
            code,
        }
    }
}

pub type SnippetList = Rc<RefCell<Vec<Snippet>>>;

pub fn load_snippets() -> SnippetList {
    // For now, start with some example snippets
    let snippets = vec![
        Snippet::new(
            "Hello World in Rust".to_string(),
            "rust".to_string(),
            r#"fn main() {
    println!("Hello, world!");
}"#.to_string(),
        ),
        Snippet::new(
            "Python List Comprehension".to_string(),
            "python".to_string(),
            r#"# Square all even numbers
squares = [x**2 for x in range(10) if x % 2 == 0]
print(squares)"#.to_string(),
        ),
        Snippet::new(
            "JavaScript Arrow Function".to_string(),
            "javascript".to_string(),
            r#"const greet = (name) => {
    return `Hello, ${name}!`;
};

console.log(greet("World"));"#.to_string(),
        ),
    ];

    Rc::new(RefCell::new(snippets))
}

pub fn save_snippets(_snippets: &[Snippet]) {
    // TODO: Implement persistence to disk
    // For now, this is a placeholder
}
