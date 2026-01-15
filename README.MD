# kiwi: A lightweight game engine in Rust

Kiwi is a lightweight game engine written in Rust that I wrote for my personal projects. It aims to provide a simple and flexible framework for building 2D and 3D games, with a focus on not doing too much for you.

Kiwi sorta focuses utilizing lifetimes and `Arc` pointers to manage component storage and access, avoiding the need for complex entity-component-system (ECS) architectures. This makes it easier to reason about your game's state and behavior. Kiwi relies on the lifetime of references to ensure safe access to components, and to make that easier Kiwi provides a couple abstractions for working with components.

hi yea this also requires nightly :/


## Features

- `ComponentStore`: Typed storage for game components.
- `WgpuRenderer`: Abstraction over the wgpu graphics API. Introduces typed buffers.
- `RenderController`: A trait based rendering pipeline.