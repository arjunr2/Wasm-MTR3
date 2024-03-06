use cmake;

fn main() {
    let dst = cmake::build("../wasm-instrument");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=stdc++");
}
