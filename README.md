# Drax

The invisible transport layer between bytes and processed frames.

## Examples

Please see the examples directory for examples of how to use the project.

To execute an example you can run either `cargo run -p <example>` or
`just example <example>`.

## Future Plans

- Add derive macros to automatically derive the `PacketComponent` trait.
- Add documentation in README.md for creating custom packets.
- Add derive documentation in the README.md.
- Build documentation surrounding context building and using context with packets.

## Packet Framing

Packet framing with Drax is not traditional. The packets are not sized then read into a buffer, but instead
are read directly into the types which need to be built out. This allows for a more efficient and flexible
system.

When you're sending a packet you're actually sending a "struct" or "enum" as the frame of the data. Since rust's type
system provides a type framing system it makes sense to port from it.

### Encryption and Compression

Encryption and compression are not currently supported by Drax.

It is a planned feature to add basic compression and encryption support.

When compression is enabled the framing will require a separate stage and header to determine the size of
the compressed data.

Encryption will not require separate framing, this means compression will be a substantial addition to the work
done during the decoding process.

## Defining a Protocol

Defining a protocol is a sane first step to building a server/client pair. Including some sort of heartbeat system
there should be a list of packets expected to be sent and received during different phases of the protocol.

The most common way to define a protocol is to use an enum with each variant representing a separate packet. Encoding
and decoding a packet should be very simple, any additional manual logic should be written into a separate struct
and referenced by the enum.
