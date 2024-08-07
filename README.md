# HTTP Proxy Service BLE
An HTTP Proxy Service (HPS) that runs on Bluetooth Low Energy (BLE) peripherals. \
It's build upon [Bluer library](https://github.com/bluez/bluer), so make sure to match the [these requirements](https://github.com/bluez/bluer?tab=readme-ov-file#requirements) before proceding.

## GATT Characteristics
The GATT characteristics of this service are based on [HPS specifications from Bluetooth standards](https://www.bluetooth.com/specifications/specs/http-proxy-service-1-0/). \
Unfortunately, this standards are limiting the size of the headers and body of each HTTP to the MTU payload size and sometimes this is not enough (iOS supports only up to 128 bytes, while Android up to 512 bytes). \
This is why, in this service, there are two new characteristics that at leasts supports the read of an HTTP response that exceeds the MTU payload size by dividing them by chunks:

* __Object Size__: combines the sizes of response headers and body into a single value. \
    The response array is 8 byte long and the first 4 bytes represents the size of the headers in bytes with the format UINT32 LE; the last 4 bytes represents the size of the body in bytes with the format UINT32 LE.

* __User Index__: combines the indexes of the chucks that must be read for both headers and body. \
    The payload in read/write must be always an 8 byte array, where the first 4 bytes must represents the index of the chunk for the headers with the format UINT32 LE, while the last 4 bytes must represents the index of the chunk for the body with the format UINT32 LE.

## Build
This project supports the build for different architectures by using docker buildx feature.
To build the project:

* Create an empty folder called `target`.

* Run this command: \
    `docker buildx build --platform <YOUR_ARCH> --no-cache --output=./target --target=artifacts -t hps-ble .`
    * The architectures could be something like: `linux/amd64`, `linux/arm64`, `linux/arm/v7`, `linux/arm/v8`.
