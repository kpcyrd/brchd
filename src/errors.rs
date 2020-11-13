pub use anyhow::{anyhow, bail, format_err, Context, Error, Result};
pub use log::{trace, debug, info, warn, error};

#[cfg(feature="httpd")]
mod web {
    use std::fmt;

    pub struct WebError {
        err: anyhow::Error,
    }

    impl fmt::Debug for WebError {
        fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
            self.err.fmt(w)
        }
    }
    impl fmt::Display for WebError {
        fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
            self.err.fmt(w)
        }
    }

    impl actix_web::error::ResponseError for WebError {
    }

    impl From<anyhow::Error> for WebError {
        fn from(err: anyhow::Error) -> WebError {
            WebError { err }
        }
    }
}
#[cfg(feature="httpd")]
pub use self::web::WebError;
