fn main() {
    // Embedded resources must be rebuilt when instructions or references change.
    println!("cargo:rerun-if-changed=skills/maintainable-code");
    println!("cargo:rerun-if-env-changed=MAINTAINABLE_SOURCE_REVISION");
    println!("cargo:rerun-if-env-changed=MAINTAINABLE_RELEASE_REPOSITORY");
}
