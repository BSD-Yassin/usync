use crate::backend::traits::Backend;
use crate::backend::local::LocalBackend;
use crate::backend::cli::ssh::SshBackend;
use crate::backend::cli::s3::S3Backend;
use crate::backend::cli::http::HttpBackend;
use crate::protocol::{Path as ProtocolPath, Protocol};

pub enum BackendInstance {
    Local(Box<dyn Backend>),
    Ssh(Box<dyn Backend>),
    S3(Box<dyn Backend>),
    Http(Box<dyn Backend>),
}

impl BackendInstance {
    pub fn as_backend(&self) -> &dyn Backend {
        match self {
            BackendInstance::Local(b) => b.as_ref(),
            BackendInstance::Ssh(b) => b.as_ref(),
            BackendInstance::S3(b) => b.as_ref(),
            BackendInstance::Http(b) => b.as_ref(),
        }
    }
}

pub fn create_backend(path: &ProtocolPath) -> Result<BackendInstance, String> {
    match path {
        ProtocolPath::Local(_) => {
            Ok(BackendInstance::Local(Box::new(LocalBackend::new())))
        }
        ProtocolPath::Remote(remote_path) => {
            match remote_path.protocol {
                Protocol::Ssh | Protocol::Sftp => {
                    Ok(BackendInstance::Ssh(Box::new(SshBackend::new(remote_path.clone()))))
                }
                Protocol::S3 => {
                    Ok(BackendInstance::S3(Box::new(S3Backend::new(remote_path.clone()))))
                }
                Protocol::Http | Protocol::Https => {
                    Ok(BackendInstance::Http(Box::new(HttpBackend::new(remote_path.clone()))))
                }
                _ => Err(format!("Unsupported protocol: {}", remote_path.protocol)),
            }
        }
    }
}

