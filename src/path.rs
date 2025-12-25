use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalPath {
    path: PathBuf,
}

impl LocalPath {
    pub fn parse(path_str: &str) -> Result<Self, PathError> {
        if path_str.contains("://") {
            let protocol = path_str.split("://").next().unwrap();
            return Err(PathError::ProtocolNotAllowed(protocol.to_string()));
        }

        if path_str.contains('@') && path_str.contains(':') {
            let parts: Vec<&str> = path_str.split('@').collect();
            if parts.len() == 2 {
                let after_at = parts[1];
                if after_at.contains(':') && !after_at.starts_with("//") {
                    return Err(PathError::RemotePathNotAllowed);
                }
            }
        }

        let path = PathBuf::from(path_str);

        Ok(LocalPath { path })
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    pub fn to_string_lossy(&self) -> std::borrow::Cow<str> {
        self.path.to_string_lossy()
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    ProtocolNotAllowed(String),
    RemotePathNotAllowed,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::ProtocolNotAllowed(protocol) => {
                write!(
                    f,
                    "Protocol '{}' is not allowed. Only local paths are supported.",
                    protocol
                )
            }
            PathError::RemotePathNotAllowed => {
                write!(f, "Remote paths (e.g., user@host:path) are not allowed. Only local paths are supported.")
            }
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_path() {
        let path = LocalPath::parse("./test.txt").unwrap();
        assert_eq!(path.to_string_lossy(), "./test.txt");
    }

    #[test]
    fn test_parse_absolute_path() {
        let path = LocalPath::parse("/tmp/test.txt").unwrap();
        assert_eq!(path.to_string_lossy(), "/tmp/test.txt");
    }

    #[test]
    fn test_reject_protocol() {
        let result = LocalPath::parse("http://example.com/file.txt");
        assert!(result.is_err());
        if let Err(PathError::ProtocolNotAllowed(proto)) = result {
            assert_eq!(proto, "http");
        } else {
            panic!("Expected ProtocolNotAllowed error");
        }
    }

    #[test]
    fn test_reject_ssh_style_remote() {
        let result = LocalPath::parse("user@host:/path/to/file");
        assert!(result.is_err());
        assert!(matches!(result, Err(PathError::RemotePathNotAllowed)));
    }

    #[test]
    fn test_allow_local_path_with_at() {
        let path = LocalPath::parse("./file@name.txt").unwrap();
        assert_eq!(path.to_string_lossy(), "./file@name.txt");
    }

    #[test]
    fn test_allow_windows_path() {
        let path = LocalPath::parse("C:\\Windows\\file.txt").unwrap();
        assert!(path.to_string_lossy().contains("file.txt"));
    }
}
