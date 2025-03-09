// SPDX-FileCopyrightText: 2025 László Vaskó <vlaci@fastmail.com>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    cell::{OnceCell, RefCell},
    thread,
};

use eyre::{eyre, Result};

pub(crate) struct SharedHandle<T> {
    handle: RefCell<Option<thread::JoinHandle<Result<T>>>>,
    result: OnceCell<Result<T>>,
}

impl<T> SharedHandle<T> {
    pub(crate) fn new(handle: thread::JoinHandle<Result<T>>) -> Self {
        Self {
            handle: RefCell::new(Some(handle)),
            result: OnceCell::new(),
        }
    }

    pub(crate) fn get_result(&self) -> Result<&T> {
        self.result
            .get_or_init(|| self.handle.take().unwrap().join().expect("Thread panicked"))
            .as_ref()
            .map_err(|err| eyre!(err.to_string()))
    }
}
