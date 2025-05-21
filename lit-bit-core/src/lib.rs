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

pub mod core;

// Re-export key types/traits for easier use by consumers of the crate.
pub use core::DefaultContext;
pub use core::MAX_ACTIVE_REGIONS;
pub use core::MachineDefinition; // If users need to construct this manually
pub use core::Runtime; // If users need to construct this manually
pub use core::StateNode; // If users need to construct this manually
pub use core::Transition; // If users need to construct this manually // Re-export this const

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

    type Event: Clone
        + PartialEq
        + Eq
        + ::core::hash::Hash
        + ::core::fmt::Debug // Use ::core::fmt::Debug for all builds
        + 'static;

    type Context: Clone + 'static;

    fn send(&mut self, event: &Self::Event) -> bool;
    fn state(&self) -> heapless::Vec<Self::State, MAX_ACTIVE_REGIONS>;
    fn context(&self) -> &Self::Context;
    fn context_mut(&mut self) -> &mut Self::Context;
}
