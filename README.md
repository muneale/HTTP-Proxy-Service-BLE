# Bluetooth GATT HTTP Proxy Server (HPS)

## Overview

This project implements a Bluetooth GATT HTTP Proxy Server (HPS) using Rust. It allows Bluetooth Low Energy (BLE) clients to make HTTP requests through this server, which acts as a proxy. The server exposes a set of GATT characteristics that clients can interact with to set up and execute HTTP requests.

## Features

- Bluetooth Low Energy (BLE) GATT server implementation
- Support for HTTP and HTTPS requests
- Configurable MTU size and timeout
- Chunked data transfer for large payloads
- Asynchronous operation using Tokio
- Structured logging with tracing
- Command-line interface for easy configuration

## Requirements

- Rust 1.56 or later
- Linux environment with BlueZ 5.50 or later
- Bluetooth hardware support

## Installation

1. Clone the repository:
    ```
    git clone https://github.com/muneale/HTTP-Proxy-Service-BLE.git
    cd HTTP-Proxy-Service-BLE
    ```

2. Build the project:
    ```
    mkdir target
    docker buildx build --platform linux/<ARCH> --no-cache --output=./target --target=artifacts -t build_rust .
    ```
    Make sure to replace __<ARCH>__ with one of the following: `arm64`, `amd64`, `arm/v7`.

## Usage

Run the server with default settings:
```
./target/release/hps-ble
```

Or with custom settings:
```
./target/release/hps-ble --name "My HPS" --timeout 30 --mtu 512
```

### Command-line Options

- `--name`: Set the advertised name of the Bluetooth service (default: "Logbot-HPS")
- `--timeout`: Set the HTTP request timeout in seconds (default: 60)
- `--mtu`: Override the MTU size in bytes (default: 0, which uses the established MTU size)

## Architecture

The project is structured into several modules:

- `main.rs`: Entry point of the application
- `lib.rs`: Main library interface
- `config.rs`: Configuration handling
- `app_state.rs`: Shared application state
- `error.rs`: Custom error types
- `constants.rs`: Constant values and UUIDs
- `bluetooth/`: Bluetooth-related functionality
  - `mod.rs`: Bluetooth module interface
  - `advertisement.rs`: BLE advertisement handling
  - `application.rs`: GATT application setup
  - `characteristics/`: Individual GATT characteristic implementations
- `http/`: HTTP-related functionality
  - `mod.rs`: HTTP module interface
  - `handler.rs`: HTTP request handling
- `utils/`: Utility functions
  - `mod.rs`: Utilities module interface
  - `signals.rs`: Signal handling

## GATT Characteristics

The server implements the following GATT characteristics:

1. HTTP URI (UUID: 0x2AB6)
2. HTTP Headers (UUID: 0x2AB7)
3. HTTP Status Code (UUID: 0x2AB8)
4. HTTP Entity Body (UUID: 0x2AB9)
5. HTTP Control Point (UUID: 0x2ABA)
6. HTTPS Security (UUID: 0x2ABB)
7. HTTP Headers Body Chunk Index (UUID: 0x2A9A)
8. HTTP Headers Body MTU Sizes (UUID: 0x2AC0)

### HTTP Headers Body Chunk Index and HTTP Headers Body MTU Sizes characteristics

These characteristics are not described in the official HPS document of Bluetooth standards, but they are required whenever either headers or body response exceeds the established MTU size. \
Also, the MTU option actually defines the size of each chunk and is not related to the MTU size established between the client and the server. \
The code behaviour is the following:
1. The HTTP Status Code notify the client that the request has been processed and returns a 3 byte array where:
    * Bytes 0..1 represents the HTTP response code (200, 401, ...) as u16 little endian number.
    * Byte 2 represents the u8 number that indicates if either headers or body are truncated (exceeds the MTU size).
2. The HTTP Headers Body Chunk Index has both headers and body indexes set to 0. If the response is truncated, the client must do as follow:
    1. By reading the HTTP Headers Body MTU Sizes characteristics, the client knows the headers, body and chunk sizes, hence it knows how many chuncks exists (e.g.: ceil(header size / chunk size)).
    2. Read the truncated payload and store its content into a proper structure.
    3. Updated the indexes of the characteristics HTTP Headers Body Chunk Index to read the next chunck.
    4. Iterate the steps 2 and 3 until all the chunks are been read.

### HTTP Headers Body Chunk Index Payload

The HTTP Headers Body Chunk Index payload has and must have always 8 bytes structured as follows:

1. 0..3 bytes indicates the index of current the headers chunk as u32 little endian number.
2. 4..7 bytes indicates the index of current the body chunk as u32 little endian number.

### HTTP Headers Body MTU Sizes Payload

The HTTP Headers Body MTU Sizes payload has and must have always 12 bytes structured as follows:

1. Bytes 0..3 indicates the response's headers size as u32 little endian number.
2. Bytes 4..7 indicates the response's body size as u32 little endian number.
3. Bytes 8..11 indicates the MTU size as u32 little endian number.

## HTTP Request Flow

1. Client writes the URI to the HTTP URI characteristic
2. Client writes headers to the HTTP Headers characteristic (if needed)
3. Client writes the request body to the HTTP Entity Body characteristic (if needed)
4. Client writes the appropriate command to the HTTP Control Point characteristic to initiate the request
5. Server processes the request and updates the HTTP Status Code characteristic
6. Client reads the response headers from the HTTP Headers characteristic
7. Client reads the response body from the HTTP Entity Body characteristic

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Bluer](https://github.com/bluez/bluer) - Rust Bluetooth library
- [Tokio](https://tokio.rs/) - Asynchronous runtime for Rust
- [Clap](https://clap.rs/) - Command-line argument parser for Rust

## Contact

If you have any questions or feedback, please open an issue on the GitHub repository.
