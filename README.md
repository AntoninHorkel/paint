# GPU accelerated paint app in Rust

![License - MIT or Apache-2.0](https://img.shields.io/badge/License-MIT_or_Apache--2.0-blue)

[EN](README.md)/[CZ](README-CZ.md)

## Usage

See also the [cool features section](#cool-features).

### Drawing primitives

1. **Primitive selection:** Select your desired primitive on the top bar.
2. **Place control points:** Click on the canvas to place control points.
3. **Activate edit mode:** Automatic for non-polygons after 2 points placed. Press ENTER for polygons after placing points.
4. **Adjust shape:** Drag control points to adjust shape.
5. **Finalize:** Press ENTER to render the primitive.
6. **Cancellation:** Press ESCAPE during any step to delete the current primitive.

### Other actions

1. **Action selection:** Select to erase content or to fill shapes or areas on the top bar.
2. **Perform action:** Erase by dragging cursor or fill by clicking.

## Cool features

- Draw lines, rectangles, circles or polygons, erase content and fill shapes or areas with color.
- Choose color from presets or using a color picker with transparency support.
- Customize line and outline thickness for all drawing tools.
- Anti-aliasing with a scale selection.
- Support for dashed lines with adjustable dash length and gap spacing.
- Zoom in/out with configurable speed (via scroll wheel or touchpad gestures).
- Move around canvas via mouse or touchpad.
- Disable real-time rendering preview while drawing shapes for improved performance on older hardware.
- Adjust the point grab tolerance.
- Customize UI settings to your liking.

## Build

### With Cargo

```sh
cargo b --release
./target/release/paint
```

### With Nix

```sh
nix build
./result/bin/paint
```

#### Dev-shell

```sh
nix develop
```

## Implementation details

- **Language:** Writen in [Rust](https://www.rust-lang.org/).
- **GPU acceleration:** Uses [WGPU](https://wgpu.rs/) for all GPU operations.
- **Windowing:** Relies on [winit](https://github.com/rust-windowing/winit) for window creation and management.
- **User interface:** Implements the UI using [egui](https://github.com/emilk/egui).
- **Rendering approach:** Primitives are rendered onto a texture using [signed distance functions (SDFs)](https://iquilezles.org/articles/distfunctions2d/) within a compute shader.
- **Shader language:** All shaders (including the compute shader) are written in [WGSL](https://www.w3.org/TR/WGSL/).

## License

- This project is distributed under either the terms of the [MIT License](LICENSE-MIT) or the [Apache License Version 2.0](LICENSE-APACHE) at your option.
- This project includes [images](src/icons) from the [Krita project](https://github.com/KDE/krita) licnesed under the [Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/).
