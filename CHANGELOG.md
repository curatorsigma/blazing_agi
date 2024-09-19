# 0.2.0 -> 0.3.0
- Removed `NotFastAGI` variant from `AGIParseError`
- Made `NotAStatus` variant take a `Box<AGIMessage>` to conserve stack space
- Made `AGIMessage` take `AGIVariableDump` by Box to reduce enum size.
- Added `Default` to `Router` and `Answer`

# 0.1.0 -> 0.2.0
## Rationale for this Version bump
Version 0.1.0 did not handle TcpStream properly. It assumed that Messages are always sent as whole packets, which may not be true
(and we can certainly not rely on it).
In 0.2.0, TcpStreams are handled correctly. It no longer matters how Message are split across packets.
## Breaking Changes
`crate::agiparse::AGIParseError` gained two more variants:
- `ReadError`. This signals, that it was impossible to read any bytes from a TcpStream. It is more of an IO-Error then one about parsing, but `AGIParseError` is still the best position for it in my opinion.
- `NetworkStartAfterOtherMessage`. This signals that the line `agi_network: yes\n` was sent after another complete message. This is more of a protocol error then a parsing error in the strict sense. It is currently still in `AGIParseError` and may move somewhere else in the future.
## Minor Changes
Tracing was reduced to be more easily readable, even on `TRACE` level.

