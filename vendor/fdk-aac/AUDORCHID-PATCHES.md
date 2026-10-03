# Audorchid changes to fdk-aac 0.8.0

Upstream: <https://github.com/haileys/fdk-aac-rs>, crates.io release 0.8.0.
Original MIT license retained in LICENSE. Modified on 2026-10-03.

- `src/enc.rs`: AACENC_BufDesc.bufSizes uses bytes, not i16 elements; input
  count is bounded so conversion to a signed C byte count cannot overflow.
- `Encoder::flush`: passes numInSamples = -1 to drain the native encoder.
- `EncoderError::is_eof`: exposes the native terminal condition without exposing
  the encoder's private pointer or relying on Rust layout assumptions.

The bundled Fraunhofer C codec is unmodified. Its separate license is in
../../licenses/FDK-AAC-NOTICE.txt; the exact sys source archive is included in
../../third_party/sources/fdk-aac-sys-0.5.0.crate.
