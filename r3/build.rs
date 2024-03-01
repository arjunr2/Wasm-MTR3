fn main() {
    println!("cargo:rustc-link-search=native=../wasm-instrument/build/");
}
