// Links the INSTRUMENTED oracle libprimecount.a (NOT the production ref
// build). Deliberately no `links` key and no shared deps: this crate must
// never interfere with titan-count's production linkage.
fn main() {
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/home/primecount-oracle/build");
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/home/primecount-oracle/build/lib/primesieve");
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/usr/lib");
    println!("cargo:rustc-link-search=native=/usr/lib/aarch64-linux-gnu");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-lib=primecount");
    println!("cargo:rustc-link-lib=primesieve");
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android") {
        println!("cargo:rustc-link-lib=c++_shared");
        println!("cargo:rustc-link-lib=omp");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=gomp");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
