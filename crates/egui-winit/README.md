# egui-winit

[![Latest version](https://img.shields.io/crates/v/egui-winit.svg)](https://crates.io/crates/egui-winit)
[![Documentation](https://docs.rs/egui-winit/badge.svg)](https://docs.rs/egui-winit)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [`winit`](https://crates.io/crates/winit).

The library translates winit events to egui, handled copy/paste, updates the cursor, open links clicked in egui, etc.

## Downstream winit 0.31 compatibility bridge

This fork preserves egui 0.36 APIs while VectorCraft tests native Wayland file
drops. It is not the proposed upstream egui migration, which targets current
main on branch feat/winit-031-review. Pinned dependency forks are prototypes,
not releases. See the downstream PR for the dependency map and local evidence.
