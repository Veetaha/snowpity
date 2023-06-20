fn main() {
    let lib_name = "twitter_scraper";

    cgo::Build::new()
        .change_dir("src/go")
        .package("main.go")
        .build(lib_name);

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=src/go");
}
