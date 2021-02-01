Miniserde
=========

<!-- [<img alt="github" src="https://img.shields.io/badge/github-dtolnay/miniserde_ditto-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/getditto/miniserde_ditto)
[<img alt="crates.io" src="https://img.shields.io/crates/v/miniserde_ditto.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/miniserde_ditto)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-miniserde_ditto-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/miniserde_ditto)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/dtolnay/miniserde_ditto/CI/master?style=for-the-badge" height="20">](https://github.com/dtolnay/miniserde_ditto/actions?query=branch%3Amaster)
-->

*Prototype of a data structure serialization library with several opposite
design goals from [Serde](https://serde.rs).*

As a prototype, this library is not a production quality engineering artifact
the way Serde is. At the same time, it is more than a proof of concept and
should be totally usable for the range of use cases that it targets, which is
qualified below.

```toml
[dependencies.miniserde_ditto]
git = "https://github.com/getditto/miniserde"
version = "0.2.0-dev"
```

<!-- Version requirement: rustc 1.31+ -->

### Example

```rust
use miniserde_ditto::{json, Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Example {
    code: u32,
    message: String,
}

fn main() -> miniserde_ditto::Result<()> {
    let example = Example {
        code: 200,
        message: "reminiscent of Serde".to_owned(),
    };

    let j = json::to_string(&example);
    println!("{}", j);

    let out: Example = json::from_str(&j)?;
    println!("{:?}", out);

    Ok(())
}
```

Here are some similarities and differences compared to Serde.

<!--

### Similar: Stupidly good performance

Seriously this library is way faster than it deserves to be. With very little
profiling and optimization so far and opportunities for improvement, this
library is on par with serde\_json for some use cases, slower by a factor of 1.5
for most, and slower by a factor of 2 for some. That is remarkable considering
the other advantages below.

-->

### Different: No monomorphization for way smaller binary size

There are no nontrivial generic methods. All serialization and deserialization
happens in terms of trait objects. Thus no code is compiled more than once
across different generic parameters. In contrast, serde\_json needs to stamp out
a fair amount of generic code for each choice of data structure being serialized
or deserialized.

Without monomorphization, the derived impls compile lightning fast and occupy
very little size in the executable.

### Similar: very similar API

This crates shares, API-wise, the same "entry-points" as the `serde` ecosystem:

  - ```rust
    #[cfg(feature = "miniserde")] // feature-gating magic
    use miniserde_ditto::{
        self as serde,
        cbor as serde_cbor,
        json as serde_json,
    };

    use serde::{
        Deserialize,
        Error,
        Result,
        Serialize,
    };
    use serde_cbor::{
        // from_reader, /* TODO */
        from_slice,
        to_vec,
        to_writer,
    };
    use serde_json::{
        from_str,
        to_string,
    };
    ```

This is to enable "easily" switching between the two "philosophical tradeoffs"
(speed _vs._ binary size) using compile-time flags.

### Different: Reduced design

This library does not tackle as expansive of a range of use cases as Serde does.
If your use case is not already covered, please use Serde.

### Different: ~~No~~ Less recursion

Serde depends on recursion for serialization as well as deserialization.
Every level of nesting in your data means more stack usage until eventually
you overflow the stack. Some formats set a cap on nesting depth to prevent
stack overflows and just refuse to deserialize deeply nested data.

In `miniserde_ditto::json` neither serialization nor deserialization involves
recursion. You can safely process arbitrarily nested data without being
exposed to stack overflows. Not even the Drop impl of our json `Value` type
is recursive so you can safely nest them arbitrarily.

On the other hand, `miniserde_ditto::cbor` deserialization **does use recursion**.
It is capped, so that a deeply nested object (_e.g._, 256 layers) causes a
controlled deserialization error (no stack overflow). This is by design,
since it doesn't seem possible to feature a design with:

  - simple traits;
  - no recursion;
  - no `unsafe`.

### Different: No deserialization error messages

When deserialization fails, the error type is a unit struct containing no
information. This is a legit strategy and not just laziness. If your use case
does not require error messages, good, you save on compiling and having your
instruction cache polluted by error handling code. If you do need error
messages, then upon error you can pass the same input to serde\_json to receive
a line, column, and helpful description of the failure. This keeps error
handling logic out of caches along the performance-critical codepath.

### Different: JSON & CBOR only

The same approach in this library could be made to work for other data formats,
but it is not a goal to enable that through what this library exposes.

### Different: Less customization

Serde has tons of knobs for configuring the derived serialization and
deserialization logic through attributes. Or for the ultimate level of
configurability you can handwrite arbitrarily complicated implementations of its
traits.

Miniserde provides just one attribute which is `rename`, and severely restricts
the kinds of on-the-fly manipulation that are possible in custom impls. If you
need any of this, use Serde -- it's a great library.

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
