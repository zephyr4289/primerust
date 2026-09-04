fn main() {
    // Ubuntu/Debian system install (Kim's .deb / /usr/lib builds).
    println!("cargo:rustc-link-search=native=/usr/lib/aarch64-linux-gnu");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    // Termux on-device primecount-ref build (static archives).
    // ~/primecount-ref/build/libprimecount.a + lib/primesieve/libprimesieve.a
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/home/primecount-ref/build");
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/home/primecount-ref/build/lib/primesieve");
    println!("cargo:rustc-link-search=native=/data/data/com.termux/files/usr/lib");
    // Let rustc pick static vs shared per platform (was hardcoded dylib=primecount,
    // which breaks Termux where only the static archive exists).
    println!("cargo:rustc-link-lib=primecount");
    println!("cargo:rustc-link-lib=primesieve");
    // C++ standard library: NDK uses c++_shared, desktop Linux uses stdc++.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android") {
        println!("cargo:rustc-link-lib=c++_shared");
        // LLVM OpenMP runtime (provides __kmpc_* / omp_* for libprimecount.a).
        println!("cargo:rustc-link-lib=omp");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=gomp");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
