from clux/muslrust:nightly

# the following folders are excluded from the build context
#  - .git
#  - dist
#  - scenarios
#  - scripts
#  - target

add . /volume
workdir /volume

run cargo build --release --target x86_64-unknown-linux-musl

from scratch
copy --from=0 /volume/target/x86_64-unknown-linux-musl/release/autobahnkreuz .

entrypoint ["/autobahnkreuz"]
