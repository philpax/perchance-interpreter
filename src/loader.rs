/// Generator loading trait and implementations
///
/// This module provides an async trait for loading generator sources,
/// with implementations for both filesystem-based and in-memory loading.
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Error types for generator loading
#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    NotFound(String),
    IoError(String),
    InvalidPath(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotFound(name) => write!(f, "Generator not found: {}", name),
            LoadError::IoError(msg) => write!(f, "IO error: {}", msg),
            LoadError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
        }
    }
}

impl std::error::Error for LoadError {}

/// Async trait for loading generator sources
///
/// This trait allows different implementations for fetching generator content,
/// such as from a filesystem, network, or in-memory store.
#[async_trait]
pub trait GeneratorLoader: Send + Sync {
    /// Load a generator's source code by its name/URL
    ///
    /// # Arguments
    /// * `name` - The generator name or URL (e.g., "nouns" or "my-generator")
    ///
    /// # Returns
    /// The generator's source code as a string, or an error if not found
    async fn load(&self, name: &str) -> Result<String, LoadError>;
}

/// Filesystem-based generator loader
///
/// Loads generators from a specified directory. Generator names are treated
/// as filenames (with optional `.perchance` extension).
pub struct FolderLoader {
    base_path: PathBuf,
}

impl FolderLoader {
    /// Create a new FolderLoader with the given base directory
    ///
    /// # Arguments
    /// * `base_path` - The directory containing generator files
    ///
    /// # Example
    /// ```no_run
    /// use perchance_interpreter::loader::FolderLoader;
    /// use std::path::PathBuf;
    ///
    /// let loader = FolderLoader::new(PathBuf::from("./generators"));
    /// ```
    pub fn new(base_path: PathBuf) -> Self {
        FolderLoader { base_path }
    }
}

#[async_trait]
impl GeneratorLoader for FolderLoader {
    async fn load(&self, name: &str) -> Result<String, LoadError> {
        // Sanitize the name to prevent path traversal attacks
        let sanitized = name.replace("..", "").replace('/', "").replace('\\', "");

        if sanitized.is_empty() {
            return Err(LoadError::InvalidPath(name.to_string()));
        }

        // Try with and without .perchance extension
        let paths = vec![
            self.base_path.join(format!("{}.perchance", sanitized)),
            self.base_path.join(&sanitized),
        ];

        for path in paths {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => return Ok(content),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(LoadError::IoError(e.to_string())),
            }
        }

        Err(LoadError::NotFound(name.to_string()))
    }
}

/// In-memory generator store
///
/// Stores generators in memory, useful for testing and embedding generators
/// directly in the application.
#[derive(Clone)]
pub struct InMemoryLoader {
    generators: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryLoader {
    /// Create a new empty InMemoryLoader
    ///
    /// # Example
    /// ```
    /// use perchance_interpreter::loader::InMemoryLoader;
    ///
    /// let loader = InMemoryLoader::new();
    /// ```
    pub fn new() -> Self {
        InMemoryLoader {
            generators: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a generator to the store
    ///
    /// # Arguments
    /// * `name` - The generator name/identifier
    /// * `source` - The generator's source code
    ///
    /// # Example
    /// ```
    /// use perchance_interpreter::loader::InMemoryLoader;
    ///
    /// let loader = InMemoryLoader::new();
    /// loader.add("nouns", "noun\n\tdog\n\tcat\n\noutput\n\t[noun]\n");
    /// ```
    pub fn add(&self, name: impl Into<String>, source: impl Into<String>) {
        let mut generators = self.generators.write().unwrap();
        generators.insert(name.into(), source.into());
    }

    /// Remove a generator from the store
    ///
    /// # Arguments
    /// * `name` - The generator name to remove
    ///
    /// # Returns
    /// `true` if the generator was removed, `false` if it didn't exist
    pub fn remove(&self, name: &str) -> bool {
        let mut generators = self.generators.write().unwrap();
        generators.remove(name).is_some()
    }

    /// Clear all generators from the store
    pub fn clear(&self) {
        let mut generators = self.generators.write().unwrap();
        generators.clear();
    }

    /// Check if a generator exists in the store
    ///
    /// # Arguments
    /// * `name` - The generator name to check
    pub fn contains(&self, name: &str) -> bool {
        let generators = self.generators.read().unwrap();
        generators.contains_key(name)
    }
}

impl Default for InMemoryLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GeneratorLoader for InMemoryLoader {
    async fn load(&self, name: &str) -> Result<String, LoadError> {
        let generators = self.generators.read().unwrap();
        generators
            .get(name)
            .cloned()
            .ok_or_else(|| LoadError::NotFound(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_loader_basic() {
        let loader = InMemoryLoader::new();
        loader.add("test", "output\n\thello\n");

        let result = loader.load("test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "output\n\thello\n");
    }

    #[tokio::test]
    async fn test_in_memory_loader_not_found() {
        let loader = InMemoryLoader::new();
        let result = loader.load("nonexistent").await;
        assert!(matches!(result, Err(LoadError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_in_memory_loader_multiple_generators() {
        let loader = InMemoryLoader::new();
        loader.add("gen1", "output\n\tfirst\n");
        loader.add("gen2", "output\n\tsecond\n");

        assert_eq!(loader.load("gen1").await.unwrap(), "output\n\tfirst\n");
        assert_eq!(loader.load("gen2").await.unwrap(), "output\n\tsecond\n");
    }

    #[tokio::test]
    async fn test_in_memory_loader_remove() {
        let loader = InMemoryLoader::new();
        loader.add("test", "output\n\thello\n");

        assert!(loader.contains("test"));
        assert!(loader.remove("test"));
        assert!(!loader.contains("test"));
        assert!(!loader.remove("test")); // Second remove returns false
    }

    #[tokio::test]
    async fn test_in_memory_loader_clear() {
        let loader = InMemoryLoader::new();
        loader.add("gen1", "content1");
        loader.add("gen2", "content2");

        assert!(loader.contains("gen1"));
        assert!(loader.contains("gen2"));

        loader.clear();

        assert!(!loader.contains("gen1"));
        assert!(!loader.contains("gen2"));
    }

    #[tokio::test]
    async fn test_folder_loader_invalid_path() {
        let loader = FolderLoader::new(PathBuf::from("/nonexistent"));
        let result = loader.load("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_folder_loader_path_traversal_protection() {
        let loader = FolderLoader::new(PathBuf::from("/tmp"));
        let result = loader.load("../etc/passwd").await;
        assert!(matches!(result, Err(LoadError::NotFound(_))));
    }
}
