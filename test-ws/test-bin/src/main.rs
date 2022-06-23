fn main() {
    for (name, value) in std::env::vars() {
        println!("env {} -> {}", name, value);
    }

    println!("Hello, world!");
}
