use std::boxed::FnBox;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::{fmt, mem};

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
#[derive(Debug)]
pub struct Future<A, E>
    where A: Debug + 'static, E: Debug + 'static
{
    lock: Arc<Mutex<Cell<bool>>>, //gets set to false when Future or Setter is dropped
    result: Holder<Box<Result<A, E>>>,
    callback: Holder<Box<FnBox(Result<A, E>) -> ()>>
}

/// The mechanism by which the result of a `Future` is resolved.
pub struct FutureSetter<A, E>
    where A: Debug + 'static, E: Debug + 'static
{
    lock: Arc<Mutex<Cell<bool>>>, //gets set to false when Future or Setter is dropped
    callback: Holder<Box<FnBox(Result<A, E>) -> ()>>,
    result: Holder<Box<Result<A, E>>>
}

///
/// Create a new (`Future`, `FutureSetter`) pair, by which the `FutureSetter` is the mechanism to
/// resolve the `Future`
pub fn new<A, E>() -> (Future<A, E>, FutureSetter<A, E>)
    where A: Debug + 'static, E: Debug + 'static
{
    let future = Future {
        lock: Arc::new(Mutex::new(Cell::new(true))),
        result: Holder::new(),
        callback: Holder::new(),
    };
    let setter = FutureSetter {
        lock: future.lock.clone(),
        callback: future.callback.clone(),
        result: future.result.clone()
    };
    (future, setter)
}

///
/// Blocks until the Future resolves
/// # Panics
/// This will panic if the FutureSetter is dropped without setting the result.
pub fn await<A, E>(f: Future<A, E>) -> Result<A, E>
    where A: Debug + 'static, E: Debug + 'static
{
    let (tx, rx) = channel();
    f.resolve(move |result| {
        tx.send(result).unwrap();
    });
    rx.recv().unwrap()
}

///
/// Like `await`, but wraps the `Future`s `Result` in an additional `Result`
/// # Failures
/// Returns Err(DroppedSetterError) if the FutureSetter goes out of scope without setting the result.
pub fn await_safe<A, E>(f: Future<A, E>) -> Result<Result<A, E>, DroppedSetterError>
    where A: Debug + 'static, E: Debug + 'static
{
    let (tx, rx) = channel();
    f.resolve(move |result| {
        tx.send(result).unwrap();
    });
    rx.recv().or(Err(DroppedSetterError))
}

impl<A: Debug + 'static, E: Debug + 'static> Future<A, E> {
    /// Create a resolved successful `Future` from an `A`
    pub fn value(value: A) -> Future<A, E> {
        Future::done(Ok(value))
    }

    /// Create a resolved error `Future` from an `E`
    pub fn err(err: E) -> Future<A, E> {
        Future::done(Err(err))
    }

    /// Create a resolved `Future` from an existing Result
    pub fn done(result: Result<A, E>) -> Future<A, E> {
        let (future, setter) = new();
        setter.set_result(result);
        future
    }

    /// Transform a successful value when the transformation cannot fail.
    /// # Examples
    /// ```
    /// use future;
    /// use future::Future;
    ///
    /// let future_int: Future<i64, ()> = Future::value(0);
    /// let future_string: Future<String, ()> = future_int.map(|i| format!("{}", i));
    /// ```
    pub fn map<F, B>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> B, F: 'static,
              B: Debug + 'static
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
    ///#[derive(Debug)]
    /// struct MyError(String);
    ///
    /// let f1: Future<(), String> = Future::err(String::from("an error!"));
    /// let f2: Future<(), MyError> = f1.map_err(|err_str| MyError(err_str));
    /// ```
    pub fn map_err<F, E2>(self, f: F) -> Future<A, E2>
        where F: FnOnce(E) -> E2, F: 'static,
              E2: Debug + 'static
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
    /// let future: Future<i64, String> = Future::err(String::from("unknown"));
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
    /// #[derive(Debug)]
    /// enum MyError {
    ///     ParseError(num::ParseIntError)
    /// }
    ///
    /// impl From<num::ParseIntError> for MyError
    /// # {
    /// #    fn from(err: num::ParseIntError) -> Self { MyError::ParseError(err) }
    /// # }
    ///
    /// let f1: Future<String, MyError> = Future::value(String::from("4"));
    /// let f2: Future<i64, MyError> = f1.and_then(|s| s.parse::<i64>());
    /// assert_eq!(4, future::await(f2).unwrap());
    /// ```
    pub fn and_then<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Result<B, E2>, F: 'static,
              E2: Into<E>, E2: Debug + 'static,
              B: Debug + 'static
    {
        self.transform(|result| match result {
            Ok(a)  => f(a).map_err(E2::into),
            Err(e) => Err(e)
        })
    }

    /// Like `handle`, except when the error transformation could fail.
    pub fn rescue<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Result<A, E2>, F: 'static,
              E2: Into<E>, E2: Debug + 'static
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
              E2: Debug + 'static,
              B: Debug + 'static
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
              E2: Into<E>, E2: Debug + 'static,
              B: Debug + 'static
    {
        self.transformf(|result| match result {
            Ok(a)  => f(a).map_err(E2::into),
            Err(e) => Future::done(Err(e))
        })
    }

    /// Like `rescue`, except when the transformation returns another `Future` instead of a
    /// `Result`
    pub fn rescuef<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Future<A, E2>, F: 'static,
              E2: Into<E>, E2: Debug + 'static
    {
        self.transformf(|result| match result {
            Err(e) => f(e).map_err(E2::into),
            Ok(a) => Future::done(Ok(a))
        })
    }

    /// Like `transform`, except when the transformation returns another `Future` instead of a
    /// `Result`
    pub fn transformf<F, B, E2>(self, f: F) -> Future<B, E2>
        where F: FnOnce(Result<A, E>) -> Future<B, E2>, F: 'static,
              E2: Debug + 'static,
              B: Debug + 'static
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
        let alive = &*self.lock.lock().unwrap();

        match self.result.get() {
            Some(box result)        => f(result),
            None if alive.get() => self.callback.set(box f),
            None => {} //Setter has been dropped without setting result; drop callback
        };
    }
}

impl<A: Debug + 'static, E: Debug + 'static> Drop for Future<A, E> {
    fn drop(&mut self) {
        let alive = &*self.lock.lock().unwrap();
        alive.set(false);
    }
}

impl<A: Debug + 'static, E: Debug + 'static> FutureSetter<A, E> {
    /// Sets the result of the associated `Future`. This call will also execute any side-effects or
    /// transformations associated with the `Future`.
    pub fn set_result<E2: Into<E>>(self, result: Result<A, E2>) {
        let result = result.map_err(E2::into);
        let alive = &*self.lock.lock().unwrap();
        match self.callback.get() {
            Some(callback) => callback(result),
            None if alive.get() => self.result.set(box result),
            None => {} //Future has been dropped without setting callback; drop result
        }
    }
}

impl<A: Debug + 'static, E: Debug + 'static> Drop for FutureSetter<A, E> {
    fn drop(&mut self) {
        let alive = &*self.lock.lock().unwrap();
        alive.set(false);
    }
}

unsafe impl<A: Debug + 'static, E: Debug + 'static> Send for FutureSetter<A, E> {}

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

struct Holder<T>(Arc<RefCell<Option<T>>>);
impl<T> Holder<T> {
    fn new() -> Holder<T> {
        Holder(Arc::new(RefCell::new(None)))
    }

    fn set(&self, t: T) {
        let Holder(ref cell) = *self;
        *cell.borrow_mut() = Some(t);
    }

    fn get(&self) -> Option<T> {
        let Holder(ref cell) = *self;
        mem::replace(&mut *cell.borrow_mut(), None)
    }
}

impl<T> Debug for Holder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Holder(ref cell) = *self;
        let inner = if cell.borrow().is_some() {"Some(T)"} else {"None"};
        write!(f, "Holder({})", inner)
    }
}

impl<T> Clone for Holder<T> {
    fn clone(&self) -> Holder<T> {
        let Holder(ref cell) = *self;
        Holder(cell.clone())
    }
}
