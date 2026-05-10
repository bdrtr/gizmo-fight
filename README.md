# Gizmo Fight

Gizmo Fight is a 3D fighting game developed using the [Gizmo Engine](https://github.com/bedir-k/gizmo-engine). It features skeletal animation, complex combat mechanics, physically-based rendering (PBR), and an entity-component-system (ECS) architecture, inspired by classic 3D arcade fighters.

## Screenshots

![Gizmo Fight Menu](screenshots/menu.jpg)
![Gizmo Fight Gameplay](screenshots/gameplay.jpg)

## Features

- **3D Combat System:** Fast-paced fighting mechanics including combos, hitboxes, hurtboxes, and hit-stop effects.
- **Skeletal Animation:** Smooth character movement and attack animations loaded from GLB/GLTF assets.
- **Custom Physics:** In-house physics engine utilizing GJK/EPA algorithms for precise collision detection.
- **Advanced Rendering:** Powered by `wgpu` with support for PBR materials, dynamic lighting, and screen-space ambient occlusion (SSAO).

## Getting Started

### Prerequisites
Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

### Building and Running
To compile and play Gizmo Fight, run the following commands in your terminal. It is highly recommended to build the game in **release mode** for optimal performance and rendering speed.

```bash
# Clone the repository
git clone https://github.com/bedir-k/gizmo-fight.git

# Navigate to the project directory
cd gizmo_fight

# Run the game in release mode
cargo run --release
```

## Powered by Gizmo Engine
Gizmo Fight is built on top of the **Gizmo Engine**. For more information about the underlying engine, its high-performance ECS, and WGPU rendering pipeline, check out the [Gizmo Engine Repository](https://github.com/bedir-k/gizmo-engine).
