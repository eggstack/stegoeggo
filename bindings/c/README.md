# C binding: ABI v1

The C binding is a versioned `cdylib` interface to the `stegoeggo` Rust library.
ABI v1 is stable and append-only within major version 1. The C header is
[`include/stegoeggo.h`](include/stegoeggo.h), and the normative function,
ownership, and error contract is [`ABI-V1.md`](ABI-V1.md).

The ABI exports 90 symbols and is intended for C11 consumers. It does not
provide a C++ wrapper. Static-library distribution is not currently provided.

## Build from source

The binding is a separate Cargo workspace and is not included in the root
workspace checks. From the repository root:

```bash
cargo build --locked --release --manifest-path bindings/c/Cargo.toml
```

Use the generated header and platform-specific shared library when linking a
consumer. Consult `ABI-V1.md` for handle lifetimes, buffer ownership, error
codes, and thread-safety rules. The project does not publish the binding through
a package manager; its manual qualification workflows produce build artifacts
only.

## License

MIT, same as the Rust library.
