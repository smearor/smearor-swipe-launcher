use thiserror::Error;

#[derive(Debug, Error)]
pub enum AddAreaError {
    #[error("Area {0} already exists")]
    AreaAlreadyExists(String),
    #[error(transparent)]
    CreateAreaError(#[from] CreateAreaError),
    #[error("Overlay not set")]
    OverlayNotSetError,
}

#[derive(Debug, Error)]
pub enum RemoveAreaError {
    #[error("Area {0} not found")]
    AreaNotFound(String),
}

#[derive(Debug, Error)]
pub enum CreateAreaError {
    #[error("Area {0} not found")]
    AreaNotFound(String),
}
