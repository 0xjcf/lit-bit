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
pub use runtime::Runtime; // If users need to construct this manually
pub use runtime::SendResult; // Re-export SendResult for public use
pub use runtime::StateNode; // If users need to construct this manually
pub use runtime::Transition; // If users need to construct this manually

pub mod prelude {
    // pub use crate::StateMachine;
}

pub trait StateMachine {
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

    fn send(&mut self, event: &Self::Event) -> crate::SendResult;
    fn state(&self) -> heapless::Vec<Self::State, MAX_ACTIVE_REGIONS>;
    fn context(&self) -> &Self::Context;
    fn context_mut(&mut self) -> &mut Self::Context;
}
