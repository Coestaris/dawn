# DAWN

Little playground for experimenting with game engine architecture and rendering techniques.
All the abstractions are stored in the separate [repo](https://github.com/Coestaris/dawnlib), which can be reused in
other future projects.

Consider downloading the latest release from the [Releases](https://github.com/Coestaris/dawn/releases) page if you just want to try the project.
If you want to build it from source, follow the instructions below.

**Important note**: To properly run the Native version of the project (Windows, Linux, macOS), you 
need to place the asset container (usually `assets.dac`) in the same directory as the executable.
Otherwise, the project will not be able to find the assets and will crash.

### Building the project

To build the project, you need to clone the repo:
```bash
git clone https://github.com/Coestaris/dawn.git --recurse-submodules 
cd dawn
```

And you need to have [Rust](https://www.rust-lang.org/tools/install) toolchain installed.

##### Native

The Windows (using MSVC), Linux and macOS platforms are well tested and has no known issues.
To build the project, run:
```bash
# To run with dev-tools enabled:
cargo run -p dawn-native
# To run without dev-tools:
cargo run -p dawn-native --no-default-features --features "build_assets"
```

##### WASM

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

##### Generating assets container

You can manually generate the assets container using:
```bash
cargo run -p dawn-package -- -i ./assets -o ./assets.dac
```

### Project Structure
- `assets` - Contains assets used in the project.
- `lib` - Contains the [DAWNLib](https://github.com/Coestaris/dawnlib).
- `dbb` - Dawn-to-Blender utility used to convert .blend scenes to DAWN assets.
- `crates/app` - Main application code.
- `crates/package` - Small utility to package the assets in a project-specific way.
- `crates/native` - Boostrap code for native platforms (Windows, Linux, macOS).
  It changes the features of the app, defines I/O, panic behavior and other platform-specific stuff.
- `crates/wasm` - Boostrap code for WebAssembly platform.
- `crates/wasm-server` - Simple [axum](https://github.com/tokio-rs/axum)-based served along with some JS code to run and host the generated WASM.

### License

All the code in this repo is licensed under the MIT license.
See the [LICENSE](./LICENSE) file for details.
All the assets in the `assets` are licensed under their respective licenses.
Check the appropriate metadata files (`*.toml`) for details.