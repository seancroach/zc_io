zc_io
=====
This crate provides a zero-copy `Read` trait and a simplified `Write` trait
useful for possibly `no_std` environments.

[![Build status](https://img.shields.io/github/actions/workflow/status/seancroach/zc_io/ci.yml)](https://github.com/seancroach/zc_io/actions)
[![Crates.io](https://img.shields.io/crates/v/zc_io)](https://crates.io/crates/zc_io)

### Documentation

https://docs.rs/zc_io

### Installation

This crate works with Cargo and is on
[crates.io](https://crates.io/crates/zc_io). Add it to your `Cargo.toml` like
so:

```toml
[dependencies]
zc_io = "0.1"
```

### `no_std` crates

This crate has a feature, `std`, that is enabled by default. To use this crate
in a `no_std` context, add the following to your `Cargo.toml`:

```toml
[dependencies]
zc_io = { version = "0.1", default-features = false }
```

### License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
