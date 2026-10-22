use vergen_gitcl::{Build, Cargo, Emitter, Gitcl, Rustc};

fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    let build = Build::builder().build_timestamp(true).build();

    let git = Gitcl::builder()
        .branch(true)
        .commit_timestamp(true)
        .sha(false)
        .build();

    let rustc = Rustc::builder()
        .channel(true)
        .commit_date(true)
        .commit_hash(true)
        .host_triple(true)
        .llvm_version(true)
        .semver(true)
        .build();

    let cargo = Cargo::builder()
        .target_triple(true)
        .debug(true)
        .opt_level(true)
        .build();

    Emitter::default()
        .add_instructions(&build)
        .unwrap()
        .add_instructions(&git)
        .unwrap()
        .add_instructions(&rustc)
        .unwrap()
        .add_instructions(&cargo)
        .unwrap()
        .emit()
        .unwrap();
}
