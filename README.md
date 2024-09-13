# rft

This project is a Rust implementation of a **Robust File Transfer (RFT)** protocol, designed for reliable file transfer over UDP.

This implementation follows the RFT protocol specification outlined here: https://nstangl.github.io/robust-file-transfer/.


# Installation

Clone the repository and build it via:
```bash
cargo build --release
```
The compiled binary for the CLI will be available in the `target/release` directory.

# Usage

To start the server, use the following command with a port of you choosing:
```bash
./rft --server --port 8088
```

To transfer files use the client:
```bash
./rft --port 8088 127.0.0.1 my-dir/File1.txt my-dir/File2.txt
```
The file paths are expected to be the same for the client and the server.

The logging levels (`debug`, `error`, `warn`, `info`) can be specified via `env` variables:
```bash
RUST_LOG=warn ./rft --port 8088 127.0.0.1 my-dir/File1.txt
```

For more details, run
```bash
./rft --help
```

Happy transferring! 🚀
