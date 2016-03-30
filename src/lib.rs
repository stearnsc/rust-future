#![doc(html_root_url = "https://stearnsc.github.io/rust-future")]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(fnbox)]
#![feature(type_ascription)]

mod join;

pub use join::*;

use std::boxed::FnBox;
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::iter::FromIterator;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

/// A handle on the result of an asynchronous compution that allows for transformations and
/// side effects.
///
/// `Future<A, E>` can be thought of as an asynchrounous Result<A, E> with improved transformation
/// and side-effect mechanisms. It is intended to be used to chain transformations and then
/// eventually consumed. The two mechanisms for final consumption are blocking via `future::await`
/// or `future::await_safe`, or with a side-effecting callback, added via `resolve`,
/// `resolve_success`, or `resolve_err`.
///
/// Additionally, side-effects that don't consume the `Future` can be added via `on_completion`,
/// `on_success`, and `on_err`.
///
/// # Examples
///
/// ```
/// use future;
/// use std::thread;
///
/// let (future, setter) = future::new::<i64, String>();
/// thread::spawn(move || {
///     // some work here
///     let result: Result<i64, String> = Ok(5);
///     setter.set_result(result);
/// });
///
/// future
///     .map(|i| i + 5)
///     .map_err(|err| err + " more info")
///     .on_err(|err| println!("Got an err: {:?}", err))
///     .resolve(|answer| println!("Answer: {:?}", answer));
/// ```
pub struct Future<A, E>
    where 'static, E: 'static
{
    lock: Arc<Mutex<()>>,
    callback: Arc<RefCell<Option<Box<FnBox(Result<A, E>) -> ()>>>>,
    result: Arc<RefCell<Option<Box<Result<A, E>>>>>
}

/// The mechanism by which the result of a `Future` is resolved.
pub struct FutureSetter<A, E>
    where A: 'static, E: 'static
{
    lock: Arc<Mutex<()>>,
    callback: Arc<RefCell<Option<Box<FnBox(Result<A, E>) -> ()>>>>,
    result: Arc<RefCell<Option<Box<Result<A, E>>>>>
}

///
/// Create a new (`Future`, `FutureSetter`) pair, by which the `FutureSetter` is the mechanism to
/// resolve the `Future`
pub fn new<A, E>() -> (Future<A, E>, FutureSetter<A, E>)
    where A: 'static, E: 'static
{
    let callback = Arc::new(RefCell::new(None));
    let result   = Arc::new(RefCell::new(None));

    let future = Future {
        lock: Arc::new(Mutex::new(())),
        callback: callback.clone(),
        result: result.clone()
    };
    let setter = FutureSetter {
        lock: future.lock.clone(),
        callback: callback,
        result: result
    };
    (future, setter)
}

/// Create a resolved successful `Future` from an `A`
pub fn value<A: 'static, E: 'static>(value: A) -> Future<A, E> {
    done(Ok(value))
}

/// Create a resolved error `Future` from an `E`
pub fn err<A: 'static, E: 'static>(err: E) -> Future<A, E> {
    done(Err(err))
}

/// Create a resolved `Future` from an existing Result
pub fn done<A: 'static, E: 'static>(result: Result<A, E>) -> Future<A, E> {
    let (future, setter) = new();
    setter.set_result(result);
    future
}

///
/// Blocks until the Future resolves
/// # Panics
/// This will panic if the FutureSetter is dropped without setting the result.
pub fn await<A, E>(f: Future<A, E>) -> Result<A, E>
    where A: 'static, E: 'static
{
    await_safe(f).unwrap()
}

///
/// Like `await`, but wraps the `Future`s `Result` in an additional `Result`
/// # Failures
/// Returns Err(DroppedSetterError) if the FutureSetter goes out of scope without setting the result.
pub fn await_safe<A, E>(f: Future<A, E>) -> Result<Result<A, E>, DroppedSetterError>
    where A: 'static, E: 'static
{
    let (tx, rx) = channel();
    f.resolve(move |result| tx.send(result).unwrap());
    rx.recv().map_err(|_| DroppedSetterError)
}

/// Execute function `F` in a new thread, returning a `Future` of the result.
pub fn run<F, A, E>(f: F) -> Future<A, E>
    where F: FnOnce() -> Result<A, E> + 'static + Send,
          A: 'static,
          E: 'static
{
    let (future, setter) = new();
    thread::spawn(move || setter.set_result(f()));
    future
}

impl<A: 'static, E: 'static> Future<A, E> {
    /// Checks whether the result on the Future has been set
    /// # Examples
    /// let (future, setter) = future::new::<i64, ()>();
    /// assert(future.is_resolved() == false);
    /// setter.set_result(Ok(0));
    /// assert(future.is_resolved());
    pub fn is_resolved(&self) -> bool {
        let _lock = self.lock.lock().unwrap();
        self.result.borrow().is_some()
    }

    /// Transform a successful value when the transformation cannot fail.
    /// # Examples
    /// ```
    /// use future;
    /// use future::Future;
    ///
    /// let future_int: Future<i64, ()> = future::value(0);
    /// let future_string: Future<String, ()> = future_int.map(|i| format!("{}", i));
    /// ```
    pub fn map<F, B>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> B, F: 'static,
              B: 'static
    {
        self.transform(|result| match result {
            Ok(a)  => Ok(f(a)),
            Err(e) => Err(e)
        })
    }

    /// Transform an error value into another.
    /// # Examples
    /// ```
    /// use future;
    /// use future::Future;
    ///
    /// # #[derive(Debug)]
    /// struct MyError(String);
    ///
    /// let f1: Future<(), String> = future::err(String::from("an error!"));
    /// let f2: Future<(), MyError> = f1.map_err(|err_str| MyError(err_str));
    /// ```
    pub fn map_err<F, E2>(self, f: F) -> Future<A, E2>
        where F: FnOnce(E) -> E2, F: 'static,
              E2: 'static
    {
        self.transform(|result| match result {
            Err(e) => Err(f(e)),
            Ok(a) => Ok(a)
        })
    }

    /// Transform an error value into a success value.
    /// # Examples
    /// ```
    /// use future;
    /// use future::Future;
    ///
    /// let future: Future<i64, String> = future::err(String::from("unknown"));
    /// let handled_future = future.handle(|err| {
    ///     if err == "unknown" {
    ///         -1
    ///     } else {
    ///         0
    ///     }
    /// });
    /// assert_eq!(-1, future::await(handled_future).unwrap());
    /// ```
    pub fn handle<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> A, F: 'static
    {
        self.transform(|result| match result {
            Err(e) => Ok(f(e)),
            Ok(a)  => Ok(a)
        })
    }

    /// Transform a success value when the transformation might fail. Returns an Err<E> if either
    /// the original computation or the transformation fail. The error type of the transformation
    /// must have an instance of Into<E> so that the final result has the same error type.
    /// # Examples
    /// ```
    /// use future;
    /// use future::Future;
    /// use std::num;
    ///
    /// # #[derive(Debug)]
    /// enum MyError {
    ///     ParseError(num::ParseIntError)
    /// }
    ///
    /// impl From<num::ParseIntError> for MyError
    /// # {
    /// #    fn from(err: num::ParseIntError) -> Self { MyError::ParseError(err) }
    /// # }
    ///
    /// let f1: Future<String, MyError> = future::value(String::from("4"));
    /// let f2: Future<i64, MyError> = f1.and_then(|s| s.parse::<i64>());
    /// assert_eq!(4, future::await(f2).unwrap());
    /// ```
    pub fn and_then<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Result<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static,
              B: 'static
    {
        self.transform(|result| match result {
            Ok(a)  => f(a).map_err(E2::into),
            Err(e) => Err(e)
        })
    }

    /// Like `handle`, except when the error transformation could fail.
    pub fn rescue<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Result<A, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transform(|result| match result {
            Err(e) => f(e).map_err(E2::into),
            Ok(a)  => Ok(a)
        })
    }

    /// The most general Future transformation; Transform the result of a `Future`, changing the
    /// success and error types if desired.
    pub fn transform<F, B, E2>(self, f: F) -> Future<B, E2>
        where F: FnOnce(Result<A, E>) -> Result<B, E2>, F: 'static,
              E2: 'static,
              B: 'static
    {
        let (future, setter) = new();
        self.resolve(|result| {
            setter.set_result(f(result));
        });
        future
    }

    /// Like `and_then`, except when the transformation returns another `Future` instead of a
    /// `Result`
    pub fn and_thenf<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Future<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static,
              B: 'static
    {
        self.transformf(|result| match result {
            Ok(a)  => f(a).map_err(E2::into),
            Err(e) => done(Err(e))
        })
    }

    /// Like `rescue`, except when the transformation returns another `Future` instead of a
    /// `Result`
    pub fn rescuef<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Future<A, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transformf(|result| match result {
            Err(e) => f(e).map_err(E2::into),
            Ok(a) => done(Ok(a))
        })
    }

    /// Like `transform`, except when the transformation returns another `Future` instead of a
    /// `Result`
    pub fn transformf<F, B, E2>(self, f: F) -> Future<B, E2>
        where F: FnOnce(Result<A, E>) -> Future<B, E2>, F: 'static,
              E2: 'static,
              B: 'static
    {
        let (future, setter) = new();
        self.resolve(|result_a| {
            f(result_a).resolve(|result_b| setter.set_result(result_b));
        });
        future
    }

    // Adds a side-effect that will run if the `Future` resolves into an error. The effect must take
    // a borrow of `E` as a parameter, since any error is not consumed.
    pub fn on_err<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&E) -> (), F: 'static
    {
        self.on_completion(|result| match *result {
            Err(ref e) => f(e),
            _ => {}
        })
    }

    // Adds a side-effect that will run if the `Future` resolves into a success. The effect must
    // take a borrow of `A` as a parameter, since any success value is not consumed.
    pub fn on_success<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&A) -> (), F: 'static
    {
        self.on_completion(|result| match *result {
            Ok(ref a) => f(a),
            _ => {}
        })
    }

    // Adds a side-effect that will run when the `Future` resolves regardless of outcome. The effect
    // must take a borrow of the result since the result is not consumed by this.
    pub fn on_completion<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&Result<A, E>) -> (), F: 'static
    {
        let (future, setter) = new();
        self.resolve(|result| {
            f(&result);
            setter.set_result(result);
        });
        future
    }

    /// Stores the side-effecting `f` to be run once the `Future` completes. `f` will only run if
    /// the `Future` resolves successfully; an error result will be dropped. This consumes the
    /// `Future`
    pub fn resolve_success<F>(self, f: F)
        where F: FnOnce(A) -> (), F: 'static
    {
        self.resolve(|result| match result {
            Ok(a) => f(a),
            _ => {}
        })
    }

    /// Stores the side-effecting `f` to be run once the `Future` completes. `f` will only run if
    /// the `Future` resolves unsuccessfully; a successful result will be dropped. This consumes the
    /// `Future`
    pub fn resolve_err<F>(self, f: F)
        where F: FnOnce(E) -> (), F: 'static
    {
        self.resolve(|result| match result {
            Err(e) => f(e),
            _ => {}
        })
    }

    /// Stores the side-effecting `f` to be run once the `Future` completes. This consumes the
    /// `Future`, and is the most common method of consuming the final result of a `Future`
    /// computation.
    pub fn resolve<F>(self, f: F)
        where F: FnOnce(Result<A, E>) -> (), F: 'static
    {
        let _lock = self.lock.lock().unwrap();

        let result_set = {
            self.result.borrow().is_some()
        };

        if result_set {
            let box result = unwrap_unsafe(self.result);
            f(result);
        } else {
            *self.callback.borrow_mut() = Some(box f);
            Arc::downgrade(&self.callback);
        }
    }
}

impl<A, E, F> FromIterator<Future<A, E>> for Future<F, E>
    where F: FromIterator<A>, A: 'static, E: 'static, F: 'static
{
    fn from_iter<I: IntoIterator<Item=Future<A,E>>>(iterator: I) -> Self {
        iterator.into_iter()
            .fold(value(vec![]), |acc, next| acc.and_thenf(move |mut successes| {
                next.transform(move |result| match result {
                    Ok(a) => {
                        successes.push(a);
                        Ok(successes)
                    },
                    Err(e) => Err(e)
                })
            })).map(|successes| successes.into_iter().collect::<F>())
    }
}

impl<A: 'static, E: 'static> FutureSetter<A, E> {
    /// Sets the result of the associated `Future`. This call will also execute any side-effects or
    /// transformations associated with the `Future`.
    pub fn set_result<E2: Into<E>>(self, result: Result<A, E2>) {
        let result = result.map_err(E2::into);
        let _lock = self.lock.lock().unwrap();

        let callback_set = {
            self.callback.borrow().is_some()
        };

        if callback_set {
            let callback = unwrap_unsafe(self.callback);
            callback(result);
        } else {
            *self.result.borrow_mut() = Some(box result);
            Arc::downgrade(&self.result);
        }
    }

    pub fn callback_set(&self) -> bool {
        let _lock = self.lock.lock().unwrap();
        self.callback.borrow().is_some()
    }
}

unsafe impl<A: 'static, E: 'static> Send for FutureSetter<A, E> {}

/// An Error indicating that the `FutureSetter` for the associated `Future` left scope and was
/// dropped before setting the result of the `Future`.
#[derive(Debug, Copy, Clone)]
pub struct DroppedSetterError;

impl fmt::Display for DroppedSetterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DroppedSetterError")
    }
}

impl Error for DroppedSetterError {
    fn description(&self) -> &str {
        "The FutureSetter associated with this Future has been dropped without setting a Result"
    }
}

/// Unwraps an `Arc<RefCell<Option<A>>>` into an `A`. This assumes the `Arc` has only
/// one strong reference and that the `Option` is `Some`.
fn unwrap_unsafe<A>(v: Arc<RefCell<Option<A>>>) -> A {
    match Arc::try_unwrap(v) {
        Ok(cell) => cell.into_inner().expect("Value was not set. This is a bug!"),
        Err(_) => panic!("Multiple strong references held to arc; this is a bug!")
    }
}

mod test {
    use std::cell::Cell;
    use std::sync::Arc;
    use super::*;

    #[test]
    fn await_resolves_for_finished_future() {
        let f: Future<i64, ()> = value(5);
        assert_eq!(await(f).unwrap(), 5);
    }

    #[test]
    fn resolve_executes_for_finished_future() {
        let f: Future<(), String> = err(String::from("An error"));
        let check = Arc::new(Cell::new(false));
        let check2 = check.clone();
        f.resolve(move |result| match result {
            Ok(_) => panic!("Unexpected value"),
            Err(_) => check2.set(true)
        });
        assert_eq!(check.get(), true);
    }

    #[test]
    fn resolving_on_setter_resolves_future() {
        let (future, setter) = new::<(), ()>();
        assert_eq!(future.is_resolved(), false);
        setter.set_result(Ok(()): Result<(), ()>);
        assert_eq!(future.is_resolved(), true);
        assert_eq!(await(future), Ok(()));
    }

    #[test]
    fn transformations_are_respected_when_resolved() {
        let f = value::<i64, String>(0)
            .map(|n| n + 1)
            .and_then(|n| Ok(n + 1): Result<i64, String>)
            .transform(|r| match r {
                Ok(n) => Err((n + 1).to_string()),
                Err(_) => panic!("Expected Ok, was Err")
            })
            .map_err(|s| incr_string(s))
            .rescue(|s| Err(incr_string(s)))
            .handle(|s| s.parse::<i64>().unwrap() + 1)
            .and_thenf(|n| err((n + 1).to_string()): Future<i64, String>)
            .transformf(|r| match r {
                Ok(_) => panic!("Expected Err, was Ok"),
                Err(s) => err(incr_string(s))
            })
            .rescuef(|n| value(n.parse::<i64>().unwrap() + 1): Future<i64, String>);

        assert_eq!(await(f), Ok(9));
    }

    #[test]
    fn transformations_are_respected_when_not_resolved() {
        let (future, setter) = new::<i64, String>();
        let transformed_future = future
            .map(|n| n + 1)
            .and_then(|n| Ok(n + 1): Result<i64, String>)
            .transform(|r| match r {
                Ok(n) => Err((n + 1).to_string()),
                Err(_) => panic!("Expected Ok, was Err")
            })
            .map_err(|s| incr_string(s))
            .rescue(|s| Err(incr_string(s)))
            .handle(|s| s.parse::<i64>().unwrap() + 1)
            .and_thenf(|n| err((n + 1).to_string()): Future<i64, String>)
            .transformf(|r| match r {
                Ok(_) => panic!("Expected Err, was Ok"),
                Err(s) => err(incr_string(s))
            })
            .rescuef(|n| value(n.parse::<i64>().unwrap() + 1): Future<i64, String>);

        setter.set_result(Ok(0): Result<i64, String>);
        assert_eq!(await(transformed_future), Ok(9));
    }

    fn incr_string(s: String) -> String {
        format!("{}", s.parse::<i64>().unwrap() + 1)
    }
}
