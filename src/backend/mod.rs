pub mod traits;
pub mod local;
pub mod error;
pub mod cli;
pub mod factory;

pub use traits::Backend;
pub use error::BackendError;
pub use factory::{create_backend, BackendInstance};

