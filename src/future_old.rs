use std::boxed::Box;
use std::cell::RefCell;
use std::ptr::Shared;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::sync::Mutex;

type Callback<T, E> = Box<FnOnce(&Result<T, E>) -> ()>;

pub struct Future<A, E>
     where A: 'static,
           E: 'static
{
    result: RefCell<Option<Result<A,E>>>,
    transformation: RefCell<Option<Box<FnOnce(Result<A, E>) -> ()>>>,
    callbacks: Mutex<RefCell<Vec<Callback<A,E>>>>,
    state: Mutex<RefCell<FutureState>>,
}

impl<A, E> Future<A, E> {
    pub fn new() -> (Self, FutureSetter<A, E>) {
        let future: Future<A, E> = Future::create(None);
        let setter = FutureSetter::new(&future);

        (future, setter)
    }

    pub fn done(a: A) -> Self {
        Future::create(Some(Ok(a)))
    }

    pub fn err(e: E) -> Self {
        Future::create(Some(Err(e)))
    }

    fn create(result: Option<Result<A, E>>) -> Self {
        let state = if result.is_some() {
            FutureState::Finished
        } else {
            FutureState::Waiting
        };

        Future {
            result: RefCell::new(result),
            transformation: RefCell::new(None),
            callbacks: Mutex::new(RefCell::new(vec![])),
            state: Mutex::new(RefCell::new(state))
        }
    }

    fn flatten<E2>(f: Future<Future<A, E2>, E>) -> Future<A, E>
        where E2: Into<E>, E2: 'static
    {
        let (future, setter) = Future::new();
        f.set_transformation::<>(move |outer_result| match outer_result {
            Ok(inner_future) => {
                inner_future.set_transformation(move |inner_result| {
                    setter.set_done(inner_result);
                })
            },
            Err(err) => setter.set_done(Err(err))
        });
        future
    }

    fn await(f: Future<A, E>) -> Result<A, E> {
        let (tx, rx) = channel();
        f.set_transformation(move |result| {
            tx.send(result);
        });
        rx.recv().unwrap()
    }

    pub fn map<F, B>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> B, F: 'static
    {
        self.and_then::<_, B, E>(|a| Ok(f(a)))
    }

    pub fn and_then<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Result<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transform(move |result| match result {
            Ok(a)    => f(a).map_err(E2::into),
            Err(err) => Err(err)
        })
    }

    pub fn transform<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(Result<A, E>) -> Result<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        let (future, setter) = Future::new();
        self.set_transformation(move |result| setter.set_done(f(result)));
        future
    }

    pub fn and_thenf<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(A) -> Future<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        self.transformf(move |r| match r {
            Ok(a)    => f(a).map_err(E2::into),
            Err(err) => Future::err(err)
        })
    }


    pub fn transformf<F, B, E2>(self, f: F) -> Future<B, E>
        where F: FnOnce(Result<A, E>) -> Future<B, E2>, F: 'static,
              E2: Into<E>, E2: 'static
    {
        let (future, setter) = Future::new();
        self.set_transformation(move |result| {
            setter.set_done::<E2>(Ok(f(result)));
        });
        Future::flatten(future)
    }

    pub fn map_err<F, E2>(self, f: F) -> Future<A, E2>
        where F: FnOnce(E) -> E2, F: 'static,
              E2: 'static
    {
        let (future, setter) = Future::new();
        self.set_transformation(move |result| {
            setter.set_done(result.map_err(f));
        });
        future
    }

    pub fn handle<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> A, F: 'static
    {
        self.transform::<_, A, E>(|result| match result {
            Err(err) => Ok(f(err)),
            Ok(a)    => Ok(a)
        })
    }

    pub fn or_else<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Result<A, E>, F: 'static
    {
        self.transform(|result| match result {
            Err(err) => f(err),
            Ok(a)    => Ok(a)
        })
    }

    pub fn or_elsef<F>(self, f: F) -> Future<A, E>
        where F: FnOnce(E) -> Future<A, E>, F: 'static
    {
        self.transformf(|result| match result {
            Err(err) => f(err),
            Ok(a)    => Future::done(a)
        })
    }

    fn set_transformation<F>(self, f: F)
        where F: FnOnce(Result<A, E>) -> (), F: 'static
    {
        let state_cell = self.state.lock().unwrap();
        let state = *state_cell.borrow();
        if state != FutureState::Waiting {
            let result = self.result
                .borrow()
                .expect("Future.result must be Some if Future.state is not Waiting");
            f(result);
        } else {
            *self.transformation.borrow_mut() = Some(Box::new(f));
        }

    }

    pub fn on_success<F>(&self, f: F) -> &Self
        where F: FnOnce(&A) -> (), F: 'static
    {
        self.on_completion(|r| match *r {
            Ok(ref a) => f(a),
            _         => ()
        })
    }

    pub fn on_failure<F>(&self, f: F) -> &Self
        where F: FnOnce(&E) -> (), F: 'static
    {
        self.on_completion(|r| match *r {
            Err(ref err) => f(err),
            _            => ()
        })
    }

    pub fn on_completion<F>(&self, f: F) -> &Self
        where F: FnOnce(&Result<A, E>) -> (), F: 'static
    {
        let state_cell = self.state.lock().unwrap();
        let state = state_cell.borrow();
        if *state == FutureState::Waiting {
            let callbacks_cell = self.callbacks.lock().unwrap();
            callbacks_cell.borrow_mut().push(Box::new(f));
        } else {
            let result = &self.result
                .borrow()
                .expect("Future.result must be Some if Future.state is not Waiting");
            f(result);
        }

        self
    }

    fn set_done(&self, result: Result<A,E>) -> () {
        {
            let mut state = self.state.lock().unwrap().borrow_mut();
            if *state != FutureState::Waiting {
                panic!("Multiple calls to set_done!");
            } else {
                *self.result.borrow_mut() = Some(result);
                *state = FutureState::Running;
            }
        }

        {
            let result = self.result.borrow();
            match *result {
                Some(result) => {
                    for callback in *self.callbacks.lock().unwrap().borrow() {
                        callback(&result);
                    }
                    if let Some(transformation) = *self.transformation.borrow() {
                        transformation(result);
                    }
                },
                None => panic!("Future.result cannot be None after being set")
            }
            let mut state = *self.state.lock().unwrap().borrow_mut();
            if state != FutureState::Running {
                panic!("Multiple calls to set_done!");
            } else {
                state = FutureState::Finished;
            }
        }
    }
}

pub struct FutureSetter<T, E>
    where T: 'static,
          E: 'static
{
    future: *const Future<T, E>
}

impl<T, E> FutureSetter<T, E> {
    fn new(f: &Future<T, E>) -> Self {
        FutureSetter {
            future: f as *const Future<T, E>
        }
    }

    pub fn set_done<E2>(self, r: Result<T, E2>)
        where E2: Into<E>, E2: 'static
    {
        unsafe {
            match self.future.as_ref() {
                Some(future) => future.set_done(r.map_err(E2::into)),
                None => ()
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum FutureState {
    Waiting,
    Running,
    Finished,
}
