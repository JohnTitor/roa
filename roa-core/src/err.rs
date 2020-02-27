pub use http::StatusCode;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::result::Result as StdResult;

/// Type alias for `Pin<Box<dyn 'a + Future<Output = Result<R>>>>`.
pub type ResultFuture<'a, R = ()> = Pin<Box<dyn 'a + Future<Output = Result<R>>>>;

/// Type alias for `StdResult<R, Error>`.
pub type Result<R = ()> = StdResult<R, Error>;

/// Throw an `Err(Error)`.
///
/// - `throw!(status_code)` will be expanded to `throw!(status_code, "")`
/// - `throw!(status_code, message)` will be expanded to `throw!(status_code, message, true)`
/// - `throw!(status_code, message, expose)` will be expanded to `return Err(Error::new(status_code, message, expose));`
///
/// ### Example
/// ```rust
/// use roa_core::{App, throw};
/// use roa_core::http::StatusCode;
///
/// let mut app = App::new(());
/// app.gate_fn(|mut ctx, next| async move {
///     next.await?; // throw
///     unreachable!();
///     ctx.resp_mut().status = StatusCode::OK;
///     Ok(())
/// });
/// app.end(|_ctx| async {
///     throw!(StatusCode::IM_A_TEAPOT, "I'm a teapot!"); // throw
///     unreachable!()
/// });
/// ```
#[macro_export]
macro_rules! throw {
    ($status_code:expr) => {
        $crate::throw!($status_code, "");
    };
    ($status_code:expr, $message:expr) => {
        $crate::throw!($status_code, $message, true);
    };
    ($status_code:expr, $message:expr, $expose:expr) => {
        return Err($crate::Error::new($status_code, $message, $expose));
    };
}

/// The `Error` of roa.
#[derive(Debug, Clone)]
pub struct Error {
    /// StatusCode will be responded to client if Error is thrown by the top middleware.
    /// ### Range
    /// 1xx/3xx/4xx/5xx
    ///
    /// ### Example
    /// ```rust
    /// use roa_core::{App, throw};
    /// use roa_core::http::StatusCode;
    ///
    /// let mut app = App::new(());
    /// app.gate_fn(|mut ctx, next| async move {
    ///     ctx.resp_mut().status = StatusCode::OK;
    ///     next.await // not caught
    /// });
    /// app.end(|_ctx| async {
    ///     throw!(StatusCode::IM_A_TEAPOT, "I'm a teapot!") // throw
    /// });
    /// ```
    pub status_code: StatusCode,

    /// Error kind, is inferred automatically by status code.
    pub kind: ErrorKind,

    /// Data will be written to response body if self.expose is true.
    /// StatusCode will be responded to client if Error is thrown by the top middleware.
    ///
    /// ### Example
    /// ```rust
    /// use roa_core::{App, Error};
    /// use roa_core::http::StatusCode;
    ///
    /// let mut app = App::new(());
    /// app.end(|_ctx| async {
    ///     Err(Error::new(StatusCode::IM_A_TEAPOT, "I'm a teapot!", false)) // message won't be exposed to user.
    /// });
    /// ```
    pub message: String,

    /// if message exposed.
    pub expose: bool,
}

/// Kind of Error.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ErrorKind {
    /// [[RFC7231, Section 6.2](https://tools.ietf.org/html/rfc7231#section-6.2)]
    Informational,

    /// [[RFC7231, Section 6.4](https://tools.ietf.org/html/rfc7231#section-6.4)]
    Redirection,

    /// [[RFC7231, Section 6.5](https://tools.ietf.org/html/rfc7231#section-6.5)]
    ClientError,

    /// [[RFC7231, Section 6.6](https://tools.ietf.org/html/rfc7231#section-6.6)]
    ServerError,
}

impl ErrorKind {
    fn infer(status_code: StatusCode) -> Self {
        use ErrorKind::*;
        match status_code.as_u16() / 100 {
            1 => Informational,
            3 => Redirection,
            4 => ClientError,
            5 => ServerError,
            _ => panic!(
                r"status {} cannot be thrown.
                  Please use `ctx.resp_mut().await.status = xxx` to set it.
               ",
                status_code
            ),
        }
    }
}

impl Error {
    /// Construct an error.
    pub fn new(status_code: StatusCode, message: impl ToString, expose: bool) -> Self {
        Self {
            status_code,
            kind: ErrorKind::infer(status_code),
            message: message.to_string(),
            expose,
        }
    }

    pub(crate) fn need_throw(&self) -> bool {
        self.kind == ErrorKind::ServerError
    }
}

macro_rules! internal_server_error {
    ($error:ty) => {
        impl From<$error> for Error {
            fn from(err: $error) -> Self {
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, err, false)
            }
        }
    };
}

internal_server_error!(std::io::Error);
internal_server_error!(http::Error);
internal_server_error!(futures::task::SpawnError);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> StdResult<(), std::fmt::Error> {
        f.write_str(&format!("{}: {}", self.status_code, self.message))
    }
}

impl std::error::Error for Error {}
