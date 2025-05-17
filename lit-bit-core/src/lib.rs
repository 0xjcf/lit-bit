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

pub mod prelude {
    // pub use crate::StateMachine;
}

pub trait StateMachine {
    #[cfg(not(feature = "std"))]
    type State: Copy + Clone + PartialEq + ::core::fmt::Debug;
    #[cfg(feature = "std")]
    type State: Copy + Clone + PartialEq + std::fmt::Debug;

    #[cfg(not(feature = "std"))]
    type Event: Copy + Clone + PartialEq + ::core::fmt::Debug;
    #[cfg(feature = "std")]
    type Event: Copy + Clone + PartialEq + std::fmt::Debug;

    type Context;

    fn send(&mut self, event: Self::Event) -> bool;
    fn state(&self) -> heapless::Vec<Self::State, 4>;
    fn context(&self) -> &Self::Context;
    fn context_mut(&mut self) -> &mut Self::Context;
}
