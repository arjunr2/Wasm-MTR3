use cmake;

fn main() {
    let wasm_instrument_dir = "../../wasm-instrument";
    let dst = cmake::build(wasm_instrument_dir);
    println!("cargo:rerun-if-changed={}", wasm_instrument_dir);
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=stdc++");
}
