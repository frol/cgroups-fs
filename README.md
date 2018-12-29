# cgroups-fs [![crates.io](https://meritbadge.herokuapp.com/cgroups-fs)](https://crates.io/crates/cgroups-fs) [![Documentation](https://docs.rs/cgroups-fs/badge.svg)](https://docs.rs/cgroups-fs) [![Build Status](https://travis-ci.org/frol/cgroups-fs.svg?branch=master)](https://travis-ci.org/frol/cgroups-fs)

Native Rust library for managing Linux Control Groups (cgroups).

This crate, curently, only supports the original, V1 hierarchy. You are welcome to contribute
Cgroups V2 support.

## Prior art

* [cgroups](https://crates.io/crates/cgroups) - it does too many things (e.g. creates cgroups in
  the subsystems that I don't plan to use, parses control files that I don't plan to use).

## Usage

First, add the following to your `Cargo.toml`:

```toml
[dependencies]
cgroups-fs = "1.0"
```

Next, use it in your crate:

```rust
use cgroups_fs;
```

## Examples

```rust
use cgroups_fs;

let my_cgroup = cgroups_fs::CgroupName("my-cgroup");
let my_cpu_cgroup = cgroups_fs::Cgroup::init(&my_cgroup, "cpu")?;
println!(
    "The current CPU shares in `my-cgroup` control group is {}",
    my_cpu_cgroup.get_value::<u64>("cpu.shares")
);
```

Please, find more examples in [the documentation](https://docs.rs/cgroups-fs#examples).

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.
