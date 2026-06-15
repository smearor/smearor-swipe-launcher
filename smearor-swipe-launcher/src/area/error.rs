use thiserror::Error;

#[derive(Debug, Error)]
pub enum AddAreaError {
    #[error("Area {0} already exists")]
    AreaAlreadyExists(String),
    #[error(transparent)]
    CreateAreaError(#[from] CreateAreaError),
    #[error("Overlay not set")]
    OverlayNotSetError,
    #[error(transparent)]
    MainContainerNotInitialized(#[from] MainContainerNotInitialized),
}

#[derive(Debug, Error)]
pub enum RemoveAreaError {
    #[error("Area {0} not found")]
    AreaNotFound(String),
    #[error(transparent)]
    MainContainerNotInitialized(#[from] MainContainerNotInitialized),
}

#[derive(Debug, Error)]
pub enum CreateAreaError {
    #[error("Area {0} not found")]
    AreaNotFound(String),
}

#[derive(Debug, Error)]
#[error("Main container not initialized")]
pub struct MainContainerNotInitialized;

#[derive(Debug, Error)]
#[error("Failed to initialize Main container")]
pub struct MainContainerInitializationError;
