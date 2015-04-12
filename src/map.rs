//!
//!  map.rs
//!
//!  Created by Mitchell Nordine at 02:07PM on April 12, 2015.
//!
//!


use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, TryLockError, Mutex};

/// A clonable Sender of a key and value pair.
pub struct Sender<K: Send, V: Send> {
    data: UnsafeCell<Arc<UnsafeCell<Arc<Mutex<HashMap<K, Option<V>>>>>>>,
}

impl<K: Send, V: Send> Clone for Sender<K, V> {
    fn clone(&self) -> Sender<K, V> {
        let new_data_ptr = unsafe { (*self.data.get()).clone() };
        Sender {
            data: UnsafeCell::new(new_data_ptr),
        }
    }
}

/// We need an unsafe implementation of Send because of the UnsafeCell.
unsafe impl<K, V> Send for Sender<K, V> {}

/// The Receiver was dropped and the channel was closed.
pub struct SendError<K, V>(K, V);

/// An enumeration of the different possible try_send errors.
pub enum TrySendError<K, V> {
    /// The Receiver was dropped and the channel was closed.
    ChannelClosed(K, V),
    /// The mutex is currently locked.
    WouldBlock(K, V),
}

impl<K: Send + Hash + Eq, V: Send> Sender<K, V> {

    /// Update the underlying hashmap with the given new key, value pair.
    /// May block briefly when acquiring access to the Mutex if either:
    /// - Another sender is sending data at the same time or
    /// - The receiver is receiving the values.
    pub fn send(&self, key: K, value: V) -> Result<(), SendError<K, V>> {
        unsafe {
            let data_ptr = (*self.data.get()).get();
            *data_ptr = match (*data_ptr).downgrade().upgrade() {
                Some(data) => data,
                None => return Err(SendError(key, value)),
            };
            match (*data_ptr).lock() {
                Ok(mut guard) => {
                    guard.insert(key, Some(value));
                    Ok(())
                },
                Err(_) => Err(SendError(key, value)),
            }
        }
    }

    /// Update the underlying hashmap with the given new key, value pair.
    /// Will return an error if the Mutex is currently locked.
    /// This method will never block.
    pub fn try_send(&self, key: K, value: V) -> Result<(), TrySendError<K, V>> {
        unsafe {
            let data_ptr = (*self.data.get()).get();
            *data_ptr = match (*data_ptr).downgrade().upgrade() {
                Some(data) => data,
                None => return Err(TrySendError::ChannelClosed(key, value)),
            };
            match (*data_ptr).try_lock() {
                Ok(mut guard) => {
                    guard.insert(key, Some(value));
                    Ok(())
                },
                Err(err) => match err {
                    TryLockError::Poisoned(_) => Err(TrySendError::ChannelClosed(key, value)),
                    TryLockError::WouldBlock => Err(TrySendError::WouldBlock(key, value)),
                }
            }
        }
    }

}

/// The receiver of the HashMap elements.
pub struct Receiver<K: Send, V: Send> {
    data: UnsafeCell<Arc<Mutex<HashMap<K, Option<V>>>>>,
}

/// The Sender was dropped and the channel was closed.
pub struct RecvError;

/// An enumeration of the different possible try_recv errors.
pub enum TryRecvError {
    /// The Senders were dropped and the channel was closed.
    ChannelClosed,
    /// A Sender has acquired the mutex and waiting would block the thread.
    WouldBlock,
}

impl<K: Send + Hash + Eq + Clone, V: Send> Receiver<K, V> {

    /// Receive the latest updates to the HashMap.
    /// May briefly block if a Sender is currently sending a key value pair.
    pub fn recv(&self) -> Result<Vec<(K, V)>, RecvError> {
        unsafe {
            let data_ptr = self.data.get();
            *data_ptr = match (*data_ptr).downgrade().upgrade() {
                Some(data) => data,
                None => return Err(RecvError),
            };
            match (*data_ptr).lock() {
                Ok(mut guard) => {
                    Ok(guard.iter_mut().filter_map(|(key, value)| {
                        match value.is_some() {
                            true => Some((key.clone(), value.take().unwrap())),
                            false => None,
                        }
                    }).collect())
                },
                Err(_) => Err(RecvError),
            }
        }
    }

    /// Receive the latest updates to the HashMap.
    /// Will return an error if the Mutex is currently locked.
    /// This method will never block.
    pub fn try_recv(&self) -> Result<Vec<(K, V)>, TryRecvError> {
        unsafe {
            let data_ptr = self.data.get();
            *data_ptr = match (*data_ptr).downgrade().upgrade() {
                Some(data) => data,
                None => return Err(TryRecvError::ChannelClosed),
            };
            match (*data_ptr).try_lock() {
                Ok(mut guard) => {
                    Ok(guard.iter_mut().filter_map(|(key, value)| {
                        match value.is_some() {
                            true => Some((key.clone(), value.take().unwrap())),
                            false => None,
                        }
                    }).collect())
                },
                Err(err) => match err {
                    TryLockError::Poisoned(_) => Err(TryRecvError::ChannelClosed),
                    TryLockError::WouldBlock => Err(TryRecvError::WouldBlock),
                }
            }
        }
    }

}

/// Construct a Sender Receiver pair.
pub fn channel<K: Send + Hash + Eq, V: Send>() -> (Sender<K, V>, Receiver<K, V>) {
    let arc = Arc::new(Mutex::new(HashMap::new()));
    let sender_arc = Arc::new(UnsafeCell::new(arc.clone()));
    (Sender { data: UnsafeCell::new(sender_arc), }, Receiver { data: UnsafeCell::new(arc), })
}

