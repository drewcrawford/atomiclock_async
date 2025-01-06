//SPDX-License-Identifier: MIT OR Apache-2.0

/*! Provides a non-blocking Mutex.

Where a mutex would block, we yield execution.

This can be considered an async version of [`atomiclock`](https://sealedabstract.com/code/atomiclock).
 */

use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use atomic_waker::AtomicWaker;
use logwise::perfwarn_begin;

#[derive(Debug)]
pub struct AtomicLockAsync<T> {
    lock: atomiclock::AtomicLock<T>,
    wakelist: atomiclock_spinlock::Lock<Vec<Arc<AtomicWaker>>>,
}


#[derive(Debug)]
pub struct Guard<'a, T> {
    _guard: ManuallyDrop<atomiclock::Guard<'a, T>>,
    lock: &'a AtomicLockAsync<T>,
}


#[derive(Debug)]
#[must_use]
pub struct LockFuture<'a, T> {
    lock: &'a AtomicLockAsync<T>,
    registered_waker: Option<Arc<AtomicWaker>>,
}


impl<T> AtomicLockAsync<T> {
    /**
    Creates a new lock.
*/
    pub const fn new(t: T) -> Self {
        AtomicLockAsync {
            lock: atomiclock::AtomicLock::new(t),
            wakelist: atomiclock_spinlock::Lock::new(vec![])
        }
    }


    /**
    Locks the lock if it is available, returning a guard if it is.
*/
    pub fn lock_if_available(&self) -> Option<Guard<'_, T>> {
        self.lock.lock()
            .map(|guard| Guard { _guard: ManuallyDrop::new(guard), lock: self })
    }

    /**
    Locks the lock.
*/
    pub fn lock(&self) -> LockFuture<T> {
        LockFuture{ lock: self, registered_waker: None }
    }

    /**
    Like lock, but with a performance warning.

    Use this to indicate that the use of lock is suspicious.
    */
    pub fn lock_warn(&self) -> LockWarnFuture<T> {
        LockWarnFuture{ underlying_future: self.lock(), perfwarn_interval: None }
    }

    /**
    Consumes the lock, returning the inner value.
*/
    pub fn into_inner(self) -> T {
        self.lock.into_inner()
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        unsafe{ManuallyDrop::drop(&mut self._guard)}; //release the underlying lock first
        //then wake a task.
        {
            let mut lock = self.lock.wakelist.spin_lock_warn();
            for drain in lock.drain(..) {
                drain.wake();
            }
        }

    }
}

impl<T> Guard<'_, T> {
    /**
    Accesses the underlying lock.
*/
    pub const fn lock(&self) -> &AtomicLockAsync<T> {
        self.lock
    }
}

impl<'a, T> std::future::Future for LockFuture<'a, T> {
    type Output = Guard<'a, T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match self.lock.lock.lock() {
            Some(guard) => {
                std::task::Poll::Ready(Guard{_guard: ManuallyDrop::new(guard), lock: self.lock})
            },
            None => {
                match self.registered_waker {
                    Some(ref waker) => {
                        waker.register(cx.waker());
                        Poll::Pending
                    },
                    None => {
                        let waker = Arc::new(AtomicWaker::new());
                        waker.register(cx.waker());
                        self.lock.wakelist.spin_lock_warn().push(waker.clone());
                        self.registered_waker = Some(waker);

                        Poll::Pending
                    }
                }
            }
        }
    }
}


#[derive(Debug)]
#[must_use]
pub struct LockWarnFuture<'a, T> {
    underlying_future: LockFuture<'a, T>,
    perfwarn_interval: Option<logwise::interval::PerfwarnInterval>,
}

impl<'a, T> std::future::Future for LockWarnFuture<'a, T> {
    type Output = Guard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let unchecked_mut = unsafe{self.get_unchecked_mut()};
        if let None = unchecked_mut.perfwarn_interval {
            unchecked_mut.perfwarn_interval = Some(perfwarn_begin!("AtomicLockAsync::lock"));
        }
        let underlying_future = unsafe{Pin::new_unchecked(&mut unchecked_mut.underlying_future)};
        let r = underlying_future.poll(cx);
        if let std::task::Poll::Ready(_) = r {
            unchecked_mut.perfwarn_interval.take();
        }
        r
    }
}

    /*
boilerplate notes.

1.  Clone can't be implemented without async lock
2.  Copy, similar
3.  PartialEq, Eq, hash, PartialOrd, etc. for similar reasons

 */

impl <T: Default> Default for AtomicLockAsync<T> {
    fn default() -> Self {
        AtomicLockAsync::new(T::default())
    }
}

//display, similar
//from is OK

impl <T> From<T> for AtomicLockAsync<T> {
    fn from(t: T) -> Self {
        AtomicLockAsync::new(t)
    }
}

//derefmut, deref, etc.

/*
Now let's check guard boilerplate.

Can't clone; locks are exclusive
similarly, no copy
 */


//from/into, no

//asref, asmut

impl<'a, T> AsRef<T> for Guard<'a, T> {
    fn as_ref(&self) -> &T {
        self._guard.as_ref()
    }
}

impl<'a, T> AsMut<T> for Guard<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        self._guard.as_mut()
    }
}

//deref, derefmut

impl<'a, T> std::ops::Deref for Guard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self._guard.deref()
    }
}

impl<'a, T> std::ops::DerefMut for Guard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self._guard.deref_mut()
    }
}




