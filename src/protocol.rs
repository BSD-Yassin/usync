use std::fmt;
use url::Url;

#[derive(Debug, Clone)]
pub enum Path {
    Local(crate::path::LocalPath),
    Remote(RemotePath),
}

#[derive(Debug, Clone)]
pub struct RemotePath {
    pub protocol: Protocol,
    pub url: Url,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protocol {
    Ssh,
    Sftp,
    Http,
    Https,
    File,
    Unknown(String),
}

impl Protocol {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ssh" => Protocol::Ssh,
            "sftp" => Protocol::Sftp,
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            "file" => Protocol::File,
            other => Protocol::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Protocol::Ssh => "ssh",
            Protocol::Sftp => "sftp",
            Protocol::Http => "http",
            Protocol::Https => "https",
            Protocol::File => "file",
            Protocol::Unknown(s) => s,
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn parse_path(path_str: &str) -> Result<Path, PathParseError> {
    if path_str.contains("://") {
        let url = Url::parse(path_str).map_err(|e| PathParseError::InvalidUrl {
            path: path_str.to_string(),
            error: e.to_string(),
        })?;

        let protocol = Protocol::from_str(url.scheme());
        let path = url.path().to_string();

        Ok(Path::Remote(RemotePath {
            protocol,
            url,
            path,
        }))
    } else if path_str.contains('@') && path_str.contains(':') {
        let parts: Vec<&str> = path_str.split('@').collect();
        if parts.len() == 2 {
            let after_at = parts[1];
            if after_at.contains(':') && !after_at.starts_with("//") {
                let host_path: Vec<&str> = after_at.splitn(2, ':').collect();
                if host_path.len() == 2 {
                    let user = parts[0];
                    let host = host_path[0];
                    let path = host_path[1];

                    let ssh_url = format!("ssh://{}@{}:{}", user, host, path);
                    let url = Url::parse(&ssh_url).map_err(|e| PathParseError::InvalidUrl {
                        path: path_str.to_string(),
                        error: e.to_string(),
                    })?;

                    Ok(Path::Remote(RemotePath {
                        protocol: Protocol::Ssh,
                        url,
                        path: path.to_string(),
                    }))
                } else {
                    crate::path::LocalPath::parse(path_str)
                        .map(Path::Local)
                        .map_err(|e| PathParseError::LocalPathError(e))
                }
            } else {
                crate::path::LocalPath::parse(path_str)
                    .map(Path::Local)
                    .map_err(|e| PathParseError::LocalPathError(e))
            }
        } else {
            crate::path::LocalPath::parse(path_str)
                .map(Path::Local)
                .map_err(|e| PathParseError::LocalPathError(e))
        }
    } else {
        crate::path::LocalPath::parse(path_str)
            .map(Path::Local)
            .map_err(|e| PathParseError::LocalPathError(e))
    }
}

#[derive(Debug)]
pub enum PathParseError {
    InvalidUrl { path: String, error: String },
    LocalPathError(crate::path::PathError),
}

impl fmt::Display for PathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathParseError::InvalidUrl { path, error } => {
                write!(f, "Invalid URL '{}': {}", path, error)
            }
            PathParseError::LocalPathError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for PathParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_path() {
        let result = parse_path("./test.txt");
        assert!(matches!(result, Ok(Path::Local(_))));
    }

    #[test]
    fn test_parse_http_url() {
        let result = parse_path("http://example.com/file.txt");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert_eq!(rp.protocol, Protocol::Http);
            assert_eq!(rp.path, "/file.txt");
        }
    }

    #[test]
    fn test_parse_https_url() {
        let result = parse_path("https://example.com/path/to/file.txt");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert_eq!(rp.protocol, Protocol::Https);
            assert_eq!(rp.path, "/path/to/file.txt");
        }
    }

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_path("ssh://user@host:/path/to/file");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert_eq!(rp.protocol, Protocol::Ssh);
        }
    }

    #[test]
    fn test_parse_sftp_url() {
        let result = parse_path("sftp://user@host:2222/path/to/file");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert_eq!(rp.protocol, Protocol::Sftp);
        }
    }

    #[test]
    fn test_parse_ssh_style_path() {
        let result = parse_path("user@host:/path/to/file");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert_eq!(rp.protocol, Protocol::Ssh);
            assert_eq!(rp.path, "/path/to/file");
        }
    }

    #[test]
    fn test_parse_unknown_protocol() {
        let result = parse_path("ftp://example.com/file.txt");
        assert!(matches!(result, Ok(Path::Remote(_))));
        if let Ok(Path::Remote(rp)) = result {
            assert!(matches!(rp.protocol, Protocol::Unknown(_)));
        }
    }

    #[test]
    fn test_protocol_from_str() {
        assert_eq!(Protocol::from_str("SSH"), Protocol::Ssh);
        assert_eq!(Protocol::from_str("sftp"), Protocol::Sftp);
        assert_eq!(Protocol::from_str("HTTP"), Protocol::Http);
        assert_eq!(Protocol::from_str("https"), Protocol::Https);
        assert!(matches!(Protocol::from_str("unknown"), Protocol::Unknown(_)));
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Ssh.to_string(), "ssh");
        assert_eq!(Protocol::Http.to_string(), "http");
        assert_eq!(Protocol::Https.to_string(), "https");
    }
}
