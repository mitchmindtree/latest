//!
//!  value.rs
//!
//!  Created by Mitchell Nordine at 02:00PM on March 25, 2015.
//!
//!


use std::cell::UnsafeCell;
use std::sync::{Arc, TryLockError, Mutex};

/// For sending the latest value.
pub struct Sender<T: Send> {
    data: UnsafeCell<Arc<UnsafeCell<Arc<Mutex<Option<T>>>>>>,
}

impl<T: Send> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        let new_data_ptr = unsafe { (*self.data.get()).clone() };
        Sender {
            data: UnsafeCell::new(new_data_ptr),
        }
    }
}

/// We need an unsafe implementation of Send because of the UnsafeCell.
unsafe impl<T> Send for Sender<T> where T: Send {}

/// The Receiver was dropped and the channel was closed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SendError<T>(T);

/// An enumeration of the different possible try_send errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// The Receiver was dropped and the channel was closed.
    ChannelClosed(T),
    /// The mutex is currently locked.
    WouldBlock(T),
}

impl<T: Send> Sender<T> {

    /// Send the latest value to the receiver.
    /// This may block shortly if the receiver has locked the mutex.
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        unsafe {
            let data_ptr = (*self.data.get()).get();
            // *data_ptr = match (*data_ptr).downgrade().upgrade() {
            //     Some(data) => data,
            //     None => return Err(SendError(t)),
            // };
            match (*data_ptr).lock() {
                Ok(mut guard) => *guard = Some(t),
                Err(_) => return Err(SendError(t)),
            }
            Ok(())
        }
    }

    /// Try and send the latest value to the receiver.
    /// If the mutex is currently locked by the receiver, return an Error indicating so.
    /// This method will never lock.
    pub fn try_send(&self, t: T) -> Result<(), TrySendError<T>> {
        unsafe {
            let data_ptr = (*self.data.get()).get();
            // *data_ptr = match (*data_ptr).downgrade().upgrade() {
            //     Some(data) => data,
            //     None => return Err(TrySendError::ChannelClosed(t)),
            // };
            match (*data_ptr).try_lock() {
                Ok(mut guard) => *guard = Some(t),
                Err(err) => match err {
                    TryLockError::Poisoned(_) => return Err(TrySendError::ChannelClosed(t)),
                    TryLockError::WouldBlock => return Err(TrySendError::WouldBlock(t)),
                }
            }
            Ok(())
        }
    }

}

/// For receiving the latest value if there has been an update.
pub struct Receiver<T: Send> {
    data: UnsafeCell<Arc<Mutex<Option<T>>>>,
}

/// We need an unsafe implementation of Send because of the UnsafeCell.
unsafe impl<T: Send> Send for Receiver<T> {}

/// The different kinds of possible receive errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RecvError {
    /// The Sender was dropped and the channel was closed.
    ChannelClosed,
    /// There have been no updates to the data since the last receive.
    NoNewValue,
}

/// An enumeration of the different possible try_recv errors.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    /// The Sender was dropped and the channel was closed.
    ChannelClosed,
    /// The sender has acquired the mutex and waiting would block the thread.
    WouldBlock,
    /// There have been no updates to the data since the last receive.
    NoNewValue,
}

impl<T: Send> Receiver<T> {

    /// Take the latest value if there is one, otherwise return a RecvError.
    /// This will block if the mutex is locked.
    pub fn recv(&self) -> Result<T, RecvError> {
        unsafe {
            let data_ptr = self.data.get();
            // *data_ptr = match (*data_ptr).downgrade().upgrade() {
            //     Some(data) => data,
            //     None => return Err(RecvError::ChannelClosed),
            // };
            match (*data_ptr).lock() {
                Ok(mut guard) => match guard.take() {
                    Some(t) => Ok(t),
                    None => Err(RecvError::NoNewValue),
                },
                Err(_) => Err(RecvError::ChannelClosed),
            }
        }
    }

    /// Attempt to retrieve the latest value.
    /// This will never block.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        unsafe {
            let data_ptr = self.data.get();
            // *data_ptr = match (*data_ptr).downgrade().upgrade() {
            //     Some(data) => data,
            //     None => return Err(TryRecvError::ChannelClosed),
            // };
            match (*data_ptr).try_lock() {
                Ok(mut guard) => match guard.take() {
                    Some(t) => Ok(t),
                    None => Err(TryRecvError::NoNewValue),
                },
                Err(err) => match err {
                    TryLockError::Poisoned(_) => Err(TryRecvError::ChannelClosed),
                    TryLockError::WouldBlock => Err(TryRecvError::WouldBlock),
                }
            }
        }
    }

}

/// Construct a new sender, receiver pair.
pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
    let arc = Arc::new(Mutex::new(None));
    let sender_arc = UnsafeCell::new(Arc::new(UnsafeCell::new(arc.clone())));
    (Sender { data: sender_arc }, Receiver { data: UnsafeCell::new(arc) })
}

