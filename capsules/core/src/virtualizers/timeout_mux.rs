// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::virtualizers::virtual_timer::VirtualTimer;
use core::cell::Cell;
use kernel::hil::time::TimerClient;
use kernel::hil::time::{Alarm, Ticks, Timer};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;

/// A lock that ensures multiple applications cannot incorrectly interleave
/// calls to asynchronous APIs.
///
/// The underlying implementation must not be  interrupt-driven and instead
///expose an API to check the status.
///
/// In order to operate without a heap and without requiring all submitted
/// jobs to have a 'static lifetime, this type requires all submitted jobs
/// to be the same type (the type parameter `J`). Consumers of the API
/// should be designed accordingly; if a heterogeneous collection of jobs
/// need to use a single instance of a `TimeoutMux`, one option is to
/// collect all possible job types into an `enum`.
pub struct TimeoutMux<'a, A: Alarm<'a>, J> {
    mux_timer: &'a VirtualTimer<'a, A>,
    check_freq: A::Ticks,
    queue: Queue<QueueEntry<A::Ticks, J>>,
    current: OptionalCell<QueueEntry<A::Ticks, J>>,
}

impl<'a, A: Alarm<'a>, J: Job> TimeoutMux<'a, A, J> {
    /// Create a new `TimeoutMux`.
    pub fn new(mux_timer: &'a VirtualTimer<'a, A>, check_freq: A::Ticks) -> TimeoutMux<'a, A, J> {
        TimeoutMux {
            mux_timer,
            check_freq,
            queue: Queue::new(),
            current: OptionalCell::empty(),
        }
    }

    /// Call this after `new` to start the timer.
    pub fn setup(&self) {
        self.mux_timer.repeating(self.check_freq);
    }

    /// Submit a new job to the `TimeoutMux`.
    pub fn submit_job(&self, job: J, timeout: A::Ticks) -> Result<(), (ErrorCode, J)> {
        self.queue
            .enqueue(QueueEntry {
                timeout,
                job,
                elapsed: Cell::new(0.into()),
            })
            .map_err(|entry| (ErrorCode::BUSY, entry.job))
    }

    /// Called once per alarm of the underlying timer, in `repeat` mode.
    pub fn tick(&self) {
        while self.current.is_none() {
            // Nothing running right now. If something is in the queue, run it.
            let mut next = match self.queue.dequeue() {
                None => return,
                Some(n) => n,
            };
            // If setup fails, short-circuit with the error.
            match next.job.setup() {
                Ok(()) => {
                    self.current.set(next);
                }
                Err(err) => {
                    next.job.on_complete(Err(err));
                    continue;
                }
            }
        }
        // If we get here, something is running. Check the status, and if it is
        // done, run the completion handler.
        //
        // .unwrap() would be safe here, but we use `map` to avoid a jump to
        // panic.
        self.current.take().map(|mut current| {
            match current.job.status() {
                // Still running; try again on the next tick.
                Ok(false) => {
                    let elapsed = current.elapsed.get();
                    let new_elapsed = elapsed.wrapping_add(self.check_freq);
                    // Check for overflow
                    //
                    // The `wrapping_sub`s cannot underflow, but `A::Ticks` does
                    // not necessarily implement `Sub`.
                    let max = A::Ticks::max_value();
                    let overflow = max.wrapping_sub(new_elapsed) > max.wrapping_sub(elapsed);
                    if elapsed >= current.timeout || overflow {
                        // Time's up! Call the timeout handler and leave
                        // `current` empty.
                        current.job.on_timeout();
                    } else {
                        // Still have more time. Keep running.
                        current.elapsed.set(elapsed);
                        self.current.set(current);
                    }
                }
                // Done. Call the completion handler.
                Ok(true) => current.job.on_complete(Ok(())),
                // An error happened; short-circuit.
                Err(err) => current.job.on_complete(Err(err)),
            }
        });
    }
}

// Internal state of a job that is waiting to run.
struct QueueEntry<T, J> {
    timeout: T,
    job: J,
    elapsed: Cell<T>,
}

const QUEUE_SIZE: usize = 16;

/// Alternative implementation of `kernel::collections::ring_buffer::RingBuffer`
/// that does not require `T: Copy`.
struct Queue<T> {
    ring: [OptionalCell<T>; QUEUE_SIZE],
    head: Cell<usize>,
    tail: Cell<usize>,
}

impl<T> Queue<T> {
    fn new() -> Queue<T> {
        Queue {
            ring: Default::default(),
            head: Cell::new(0),
            tail: Cell::new(0),
        }
    }

    fn has_elements(&self) -> bool {
        self.head.get() != self.tail.get()
    }

    fn is_full(&self) -> bool {
        self.head.get() == ((self.tail.get() + 1) % self.ring.len())
    }

    fn enqueue(&self, val: T) -> Result<(), T> {
        if self.is_full() {
            // Incrementing tail will overwrite head
            Err(val)
        } else {
            self.ring[self.tail.get()].set(val);
            self.tail.set((self.tail.get() + 1) % self.ring.len());
            Ok(())
        }
    }

    fn dequeue(&self) -> Option<T> {
        if self.has_elements() {
            let val = &self.ring[self.head.get()];
            self.head.set((self.head.get() + 1) % self.ring.len());
            val.take()
        } else {
            None
        }
    }
}

/// Represents jobs that can run on the underlying shared resource.
pub trait Job {
    /// Logic to run when first acquiring the lock
    fn setup(&mut self) -> Result<(), ErrorCode>;
    /// Logic that checks whether the operation is complete. Will run every
    /// interation of the Tock main loop. Returns `true` if the operation is done.
    fn status(&mut self) -> Result<bool, ErrorCode>;
    /// Logic that will run when the operation completes, i.e. `status` returns
    /// `true`.
    fn on_complete(&mut self, status: Result<(), ErrorCode>);
    /// Invoked if the operation receives a timer interrupt. Should clean up any
    /// internal state to ensure another user can safely acquire the lock.
    fn on_timeout(&self);
}

// Implement the timer client for all `Job`s.
impl<'a, A: Alarm<'a>, J: Job> TimerClient for TimeoutMux<'a, A, J> {
    fn timer(&self) {
        self.tick();
    }
}
