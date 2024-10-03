use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppContextError {
    #[error("the app object `{obj_name}` with actual type `{obj_type}` cannot be casted to `{expected_type}")]
    UnsupportedCast {
        obj_name: &'static str,
        obj_type: &'static str,
        expected_type: &'static str,
    },

    #[error("the app context is dropped")]
    AppContextDropped,

    #[error("the app object `{obj_name}` with type `{obj_type}` is not found")]
    ObjectNotFound {
        obj_name: &'static str,
        obj_type: &'static str,
    },

    #[error("unexpected error `{0}`")]
    UnexpectedError(&'static str),
}

pub type AppContextResult<T> = Result<T, AppContextError>;
