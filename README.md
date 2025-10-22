# DAWN

<b>
The development of this project has been suspended due to architectural decisions that
make further development impractical. 
Using pure OpenGL and handwritten resources management is quite exhausting for me,
and I would like to try something new.
</b>

Little playground for experimenting with game engine architecture and rendering techniques.
All the abstractions are stored in the separate [repo](https://github.com/Coestaris/dawnlib), which can be reused in
other future projects.

Consider downloading the latest release from the [Releases](https://github.com/Coestaris/dawn/releases) page if you just want to try the project.
If you want to build it from source, follow the instructions below.

**Important note**: To properly run the Native version of the project (Windows, Linux, macOS), you 
need to place the asset container (usually `assets.dac`) in the same directory as the executable.
Otherwise, the project will not be able to find the assets and will crash.

<details>
  <summary>Screenshots</summary>
  <p align="center">
     <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/1.png width="350">
     <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/2.png width="350">
     <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/3.png width="350">
     <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/4.png width="350">
     <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/5.png width="350">
  </p>
</details>

## Features

Main features:
- Pure Rust;
- Cross-platform support (Windows, Linux, macOS, WebAssembly);
- Asynchronous resource loading and support for hot-swapping;
- OpenGL 4.1;
- Entity-Component-System (ECS) architecture using [evenio](https://crates.io/crates/evenio);
- Optional development UI (devtools) using [egui](https://crates.io/crates/egui);
- Asset-based scene management;

#### Graphical Pipeline

Main features:
- Compile-time defined rendering pipeline using custom passes. The rendering pipeline is fully configurable at compile-time using custom passes.
  Each pass can have its own shaders, framebuffer attachments, and other settings.
- When not using the devtools, all the constants are embedded in the shader source;
- Implemented some popular rendering techniques:
  - Deferred PBR shading;
  - Normal mapping;
  - Half-resolution SSAO and bilateral separated blur;
  - Transparent objects sorting;
  - FXAA;
  - Skybox;
  - Devtools rendering (gizmos, wireframe, etc.);

Schematic of the current rendering pipeline:
<p align="center">
    <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/pipeline_stage1.png>
    <em>Stage 1: Geometry pass rendering to G-Buffer</em>
</p>

<p align="center">
    <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/pipeline_stage2.png>
    <em>Stage 2: Half-res SSAO and bilateral blur</em>
</p>

<p align="center">
    <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/pipeline_stage3.png>
    <em>Stage 3: Lightning pass, transparent objects, and postprocessing</em>
</p>

#### Threading model

<p align="center">
    <img src=https://github.com/Coestaris/dawn/blob/develop/assets/docs/threading_model.png>
    <em>Example of multithreaded V-synced rendering</em>
</p>

Application is capable of running in the following modes:
- Single-threaded: all the tasks are executed on the main thread;
- Multithreaded: moves all non-graphics tasks to a separate thread called 'World' thread. Also enables [rayon](https://crates.io/crates/rayon) to parallelize the world tasks in some cases;

When using multithreading, there are two modes of synchronization with the main thread:
- Two-point synchronization: World thread runs independently and synchronizes with the main thread
    at the beginning and at the end of the frame. This mode is useful when the world update time is
    predictable and does not vary much between frames. It guarantees maximum smoothness of the
    rendering but can introduce input lag if the world update takes too long.
- Free running: World thread runs independently and does not synchronize with the main thread.
All the heavy data is transferred using triple-buffer, to allow asynchronous data transfer without
blocking both threads. Smaller data is transferred using lock-free queues. 

#### Assets

Raw Assets (.png, .gltf, .glb, etc.) are converted into simple Internal Representation (IR)
to avoid heavy decoding at a runtime and to reduce the size of the executable.
IR is a GPU-friendly format that can be directly uploaded to the GPU memory without any additional processing.
Conversion is done using a crate called [dacgen](https://github.com/Coestaris/dawnlib/tree/develop/crates/dacgen).

After the assets are converted, they are packed into a DAC (Dawn Asset Container) file.
DAC file contains of TLV (Type-Length-Value) tags, which are used to store the assets and the 
TOC (Table of Contents) at the beginning of the file for fast lookup. Read more about the DAC format in the [dawnlib/dac/lib.rs](https://github.com/Coestaris/dawnlib/blob/develop/crates/dac/src/lib.rs)

The main reason of developing a custom container was to have the following features:
- Stateless O(1) assets lookup;
- Support of compression;
- Minimal overhead and code footprint;

The IR assets are stored as a serialized structure in the DAC file using [bincode](https://crates.io/crates/bincode).

## Project Structure
- `assets` - Contains assets used in the project.
- `lib` - Contains the [DAWNLib](https://github.com/Coestaris/dawnlib).
- `crates/app` - Main application code.
- `crates/package` - Small utility to package the assets in a project-specific way.
- `crates/native` - Boostrap code for native platforms (Windows, Linux, macOS).
  It changes the features of the app, defines I/O, panic behavior, and other platform-specific stuff.
- `crates/wasm` - Boostrap code for WebAssembly platform.
- `crates/wasm-server` - Simple [axum](https://github.com/tokio-rs/axum)-based served along with some JS code to run and host the generated WASM.

## License

All the code in this repo is licensed under the MIT license.
See the [LICENSE](./LICENSE) file for details.
All the assets in the `assets` are licensed under their respective licenses.
Check the appropriate metadata files (`*.toml`) for details.

## Building the project

To build the project, you need to clone the repo:
```bash
git clone https://github.com/Coestaris/dawn.git --recurse-submodules 
cd dawn
```

And you need to have [Rust](https://www.rust-lang.org/tools/install) toolchain installed.

#### Native

The Windows (using MSVC), Linux and macOS platforms are well tested and has no known issues.
To build the project, run:
```bash
# To run with dev-tools enabled:
cargo run -p dawn-native
# To run without dev-tools:
cargo run -p dawn-native --no-default-features --features "build_assets"
```

#### WASM

WASM is nearly dead and needs a lot of work to be fully functional.
For now it compiles, but frames are not being rendered correctly.
You need to have `wasm-pack` installed. You can install it using:
```bash
cargo install wasm-pack
```

And `python3` with `pip` to collect and serve the distribution files.

To build the project and collect the distribution files, run:
```bash
# With debug info and dev-tools enabled:
python3 crates/wasm-server/compose.py compose --dist crates/wasm-server/dist/ --dev
# Without debug info and dev-tools:
python3 crates/wasm-server/compose.py compose --dist crates/wasm-server/dist/
```

To run the server, use:
```bash
python3 crates/wasm-server/compose.py serve --dist crates/wasm-server/dist/
```

Then open the printed URL in your browser.

#### Generating assets container

You can manually generate the assets container using:
```bash
cargo run -p dawn-package -- -i ./assets -o ./assets.dac
```