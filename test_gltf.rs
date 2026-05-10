use gltf::Gltf;
fn main() {
    match Gltf::open("assets/main_char.glb") {
        Ok(g) => println!("Success! Nodes: {}", g.nodes().count()),
        Err(e) => println!("Failed: {:?}", e),
    }
}
