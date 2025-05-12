#![cfg_attr(not(feature = "std"), no_std)]

//! # Rust-Statechart
//! A Rust library for building type-safe, Harel statecharts, inspired by XState.
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
    fn state(&self) -> Self::State;
    fn context(&self) -> &Self::Context;
    fn context_mut(&mut self) -> &mut Self::Context;
}
