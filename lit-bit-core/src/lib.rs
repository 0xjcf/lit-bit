// Copyright 2025 0xjcf
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

//! # Rust-Statechart
//! A Rust library for building type-safe, Harel statecharts, inspired by `XState`.
//! Aims to be ergonomic, `no_std` compatible, and suitable for embedded to backend applications.

// No `use core::fmt` or `use ::core::fmt` needed here if we qualify directly in trait bounds.

pub mod runtime;

// Re-export macros from lit_bit_macro
pub use lit_bit_macro::{statechart, statechart_event};

// Re-export key types/traits for easier use by consumers of the crate.
pub use runtime::ActionFn; // Re-export function types for macro use
pub use runtime::DefaultContext;
pub use runtime::EntryExitActionFn;
pub use runtime::GuardFn;
pub use runtime::MAX_ACTIVE_REGIONS;
pub use runtime::MachineDefinition; // If users need to construct this manually
pub use runtime::ProcessingError; // Re-export ProcessingError for error handling
pub use runtime::Runtime; // If users need to construct this manually
pub use runtime::SendResult; // Re-export SendResult for public use
pub use runtime::StateNode; // If users need to construct this manually
pub use runtime::Transition; // If users need to construct this manually

// Re-export key actor types for easier access
pub use actor::address::Address;
pub use actor::backpressure::SendError;

// Re-export actor types that are always available
pub use actor::{Actor, ActorError, RestartStrategy, actor_task};

// Re-export mailbox types based on feature flags
#[cfg(feature = "async-tokio")]
pub use actor::{Inbox, Outbox, create_mailbox};

#[cfg(not(feature = "async-tokio"))]
pub use actor::{Inbox, Outbox, create_mailbox_safe};

// Note: static_mailbox macro is available directly from the crate root

pub mod prelude {
    // pub use crate::StateMachine;
}

pub mod actor;

pub trait StateMachine<const N_ACTIVE: usize = MAX_ACTIVE_REGIONS> {
    type State: Copy
        + Clone
        + PartialEq
        + Eq
        + ::core::hash::Hash
        + ::core::fmt::Debug // Use ::core::fmt::Debug for all builds
        + 'static;

    type Event: ::core::fmt::Debug // Use ::core::fmt::Debug for all builds
        + 'static; // Removed Clone, PartialEq, Eq, Hash

    type Context: Clone + 'static;

    fn send(&mut self, event: &Self::Event) -> SendResult;
    fn state(&self) -> heapless::Vec<Self::State, N_ACTIVE>;
    fn context(&self) -> &Self::Context;
    fn context_mut(&mut self) -> &mut Self::Context;
}

#[cfg(test)]
mod re_export_tests {
    //! Tests to verify that key actor types are properly re-exported at the top level

    #[test]
    fn actor_types_are_re_exported() {
        // Test that we can import actor types directly from the crate root
        use crate::{ActorError, RestartStrategy, SendError};

        // Test that the types are accessible and have the expected properties
        let error = ActorError::StartupFailure;
        let strategy = RestartStrategy::OneForOne;
        let send_error = SendError::Full(42u32);

        // Verify that the types implement expected traits
        assert_eq!(ActorError::StartupFailure, error);
        assert_eq!(RestartStrategy::OneForOne, strategy);
        assert_eq!(SendError::Full(42u32), send_error);
    }

    #[cfg(feature = "std")]
    #[test]
    fn mailbox_types_are_re_exported() {
        // Test that mailbox types and functions are re-exported
        use crate::{Inbox, Outbox, create_mailbox};

        let (_outbox, _inbox): (Outbox<u32>, Inbox<u32>) = create_mailbox::<u32>(4);

        // This test just verifies the types are accessible
        // Actual functionality is tested in the actor module tests
    }
}
