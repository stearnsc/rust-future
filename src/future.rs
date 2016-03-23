use std::boxed::FnBox;
use std::cell::Cell;
use std::sync::Arc;
use std::sync::Mutex;
use std::{mem, ptr};

pub struct Future<A, E> {
    lock: Arc<Mutex<Cell<bool>>>, //gets set to false when Future or Setter is dropped
    result: Option<Box<Result<A, E>>>,
    setter: *mut FutureSetter<A, E>
}

pub struct FutureSetter<A, E> {
    lock: Arc<Mutex<Cell<bool>>>, //gets set to false when Future or Setter is dropped
    callback: Option<Box<FnBox(Result<A, E>) -> ()>>,
    future: *mut Future<A, E>,
}

impl<A: 'static, E: 'static> Future<A, E> {
    pub fn new() -> (Future<A, E>, FutureSetter<A, E>) {
        let mut future = Future {
            lock: Arc::new(Mutex::new(Cell::new(true))),
            result: None,
            setter: ptr::null_mut(),
        };
        let mut setter = FutureSetter {
            lock: future.lock.clone(),
            callback: None,
            future: &mut future
        };
        future.setter = &mut setter;
        (future, setter)
    }

    pub fn done(result: Result<A, E>) -> Future<A, E> {
        let (future, setter) = Future::new();
        setter.set_result(result);
        future
    }

    pub fn map<F, B>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> B, F: 'static,
              B: 'static
    {
        self.transform(|result| match result {
            Ok(a)  => Ok(f(a)),
            Err(e) => Err(e)
        })
    }

    pub fn map_err<F, E2>(self, f: F) -> Future<A, E2>
        where F: FnOnce(E) -> E2, F: 'static,
              E2: 'static
    {
        self.transform(|result| match result {
            Err(e) => Err(f(e)),
            Ok(a) => Ok(a)
        })
    }

    pub fn handle<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> A, F: 'static
    {
        self.transform(|result| match result {
            Err(e) => Ok(f(e)),
            Ok(a)  => Ok(a)
        })
    }

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

    pub fn rescue<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Result<A, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transform(|result| match result {
            Err(e) => f(e).map_err(E2::into),
            Ok(a)  => Ok(a)
        })
    }

    pub fn transform<F, B, E2>(self, f: F) -> Future<B, E2>
        where F: FnOnce(Result<A, E>) -> Result<B, E2>, F: 'static,
              E2: 'static,
              B: 'static
    {
        let (future, setter) = Future::new();
        self.resolve(|result| {
            setter.set_result(f(result));
        });
        future
    }

    pub fn and_thenf<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Future<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static,
              B: 'static
    {
        self.transformf(|result| match result {
            Ok(a)  => f(a).map_err(E2::into),
            Err(e) => Future::done(Err(e))
        })
    }

    pub fn rescuef<F, E2>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Future<A, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transformf(|result| match result {
            Err(e) => f(e).map_err(E2::into),
            Ok(a) => Future::done(Ok(a))
        })
    }

    pub fn transformf<F, B, E2>(self, f: F) -> Future<B, E2>
        where F: FnOnce(Result<A, E>) -> Future<B, E2>, F: 'static,
              E2: 'static,
              B: 'static
    {
        let (future, setter) = Future::new();
        self.resolve(|result_a| {
            f(result_a).resolve(|result_b| setter.set_result(result_b));
        });
        future
    }

    pub fn on_failure<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&E) -> (), F: 'static
    {
        self.on_completion(|result| match *result {
            Err(ref e) => f(e),
            _ => {}
        })
    }

    pub fn on_success<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&A) -> (), F: 'static
    {
        self.on_completion(|result| match *result {
            Ok(ref a) => f(a),
            _ => {}
        })
    }

    pub fn on_completion<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(&Result<A, E>) -> (), F: 'static
    {
        let (future, setter) = Future::new();
        self.resolve(|result| {
            f(&result);
            setter.set_result(result);
        });
        future
    }

    pub fn resolve<F>(mut self, f: F)
        where F: FnOnce(Result<A, E>) -> (), F: 'static
    {
        let alive = &*self.lock.lock().unwrap();

        match mem::replace(&mut self.result, None) {
            Some(result) => {
                let box result = result;
                f(result);
            },
            None if alive.get() => unsafe {
                self.setter.as_mut().unwrap().callback = Some(box f);
            },
            None => {} //Setter has been dropped without setting result; drop callback
        };
    }
}

impl<A, E> Drop for Future<A, E> {
    fn drop(&mut self) {
        let alive = &*self.lock.lock().unwrap();
        alive.set(false);
    }
}

impl<A: 'static, E: 'static> FutureSetter<A, E> {
    pub fn set_result<E2: Into<E>>(mut self, result: Result<A, E2>) {
        let result = result.map_err(E2::into);
        let alive = &*self.lock.lock().unwrap();
        match mem::replace(&mut self.callback, None) {
            Some(callback) => {
                callback(result);
            },
            None if alive.get() => unsafe {
                self.future.as_mut().unwrap().result = Some(box result);
            },
            None => {} //Future has been dropped without setting callback; drop result
        }
    }
}

impl<A, E> Drop for FutureSetter<A, E> {
    fn drop(&mut self) {
        let alive = &*self.lock.lock().unwrap();
        alive.set(false);
    }
}
