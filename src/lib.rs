/*! Provides a non-blocking Mutex.

Where a mutex would block, we yield execution.

This can be considered an async version of `atomiclock`.
 */

use std::pin::Pin;


#[derive(Debug)]
pub struct AtomicLockAsync<T> {
    lock: atomiclock::AtomicLock<T>,
    wakelist: wakelist::WakeList,
}



#[derive(Debug)]
pub struct Guard<'a, T> {
    _guard: atomiclock::Guard<'a, T>,
}

struct Future<'a, T> {
    lock: &'a AtomicLockAsync<T>,
}



impl<T> AtomicLockAsync<T> {
    pub const fn new(t: T) -> Self {
        AtomicLockAsync {
            lock: atomiclock::AtomicLock::new(t),
            wakelist: wakelist::WakeList::new(),
        }
    }

    /**
    Locks the lock if it is available, returning a guard if it is.
*/
    pub fn lock_if_available(&self) -> Option<Guard<'_, T>> {
        self.lock.lock()
            .map(|guard| Guard { _guard: guard })
    }

    pub async fn lock(&self) -> Guard<'_, T> {
        Future{ lock: self }.await
    }
}

impl<'a, T> std::future::Future for Future<'a, T> {
    type Output = Guard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.lock.wakelist.push(cx.waker().clone());
        let guard = self.lock.lock.lock();
        if let Some(guard) = guard {
            std::task::Poll::Ready(Guard { _guard: guard })
        } else {
            std::task::Poll::Pending
        }
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
partialEq/Eq are ok
 */

impl<'a, T: PartialEq> PartialEq for Guard<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self._guard.eq(&other._guard)
    }
}

impl<'a, T: Eq> Eq for Guard<'a, T> {}

//partialOrd, Ord,

impl<'a, T: PartialOrd> PartialOrd for Guard<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self._guard.partial_cmp(&other._guard)
    }
}

impl<'a, T: Ord> Ord for Guard<'a, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self._guard.cmp(&other._guard)
    }
}

//hash

impl<'a, T: std::hash::Hash> std::hash::Hash for Guard<'a, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._guard.hash(state)
    }
}

//default, no
//display
impl <'a, T: std::fmt::Display> std::fmt::Display for Guard<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self._guard.fmt(f)
    }
}

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




