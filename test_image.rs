fn main() {
    match image::open("assets/arena_floor.jpg") {
        Ok(_) => println!("Successfully opened!"),
        Err(e) => println!("Error: {}", e),
    }
}
