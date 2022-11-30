use vergen::vergen;

fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    let mut cfg = vergen::Config::default();

    *cfg.git_mut().commit_timestamp_kind_mut() = vergen::TimestampKind::DateAndTime;
    *cfg.build_mut().kind_mut() = vergen::TimestampKind::DateAndTime;

    vergen(cfg).unwrap();
}
