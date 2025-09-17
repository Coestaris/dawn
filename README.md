# DAWN

Little playground for experimenting with game engine architecture and rendering techniques.
All the abstractions are stored in the separate [repo](https://github.com/Coestaris/dawnlib), which can be reused in
other future projects.

#### Project Structure
- `assets` - Contains assets used in the project. 
- `lib` - Contains the DAWNLib.
- `dbb` - Dawn-to-Blender utility used to convert .blend scenes to DAWN assets.
- `crates/app` - Main application code.
- `crates/package` - Small utility to package the assets in a project-specific way. 
- `crates/native` - Boostrap code for native platforms (Windows, Linux, macOS). 
  It changes the features of the app, defines I/O, panic behavior and other platform-specific stuff.
- `crates/wasm` - Boostrap code for WebAssembly platform.
- `crates/wasm-server` - Simple axum-based served along with some JS code to run and host the generated WASM.
