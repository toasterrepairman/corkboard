use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub language: String,
    pub code: String,
}

impl Snippet {
    pub fn new(title: String, language: String, code: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            language,
            code,
        }
    }

    pub fn get_searchable_text(&self) -> String {
        format!("{} {} {}", self.title, self.language, self.code)
    }
}

pub type SnippetList = Rc<RefCell<Vec<Snippet>>>;

fn get_snippets_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("corkboard");
    path.push("snippets.json");
    path
}

fn get_default_snippets() -> Vec<Snippet> {
    vec![
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
    ]
}

pub fn load_snippets() -> SnippetList {
    let path = get_snippets_path();

    let snippets = if path.exists() {
        // Load from disk
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(snippets) => {
                        println!("Loaded snippets from {:?}", path);
                        snippets
                    }
                    Err(e) => {
                        eprintln!("Failed to parse snippets file: {}, using defaults", e);
                        get_default_snippets()
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read snippets file: {}, using defaults", e);
                get_default_snippets()
            }
        }
    } else {
        // First run, use defaults
        println!("No snippets file found, using default examples");
        let defaults = get_default_snippets();
        // Save the defaults so they persist
        if let Err(e) = save_snippets(&defaults) {
            eprintln!("Failed to save default snippets: {}", e);
        }
        defaults
    };

    Rc::new(RefCell::new(snippets))
}

pub fn save_snippets(snippets: &[Snippet]) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_snippets_path();

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(snippets)?;
    std::fs::write(path, json)?;
    Ok(())
}
