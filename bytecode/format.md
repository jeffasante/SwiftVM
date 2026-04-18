# Swift Bytecode (`.swiftbc`) Current Draft

Implemented in `crates/vm-core/src/bytecode.rs`.

## Version

- Magic: `SWBC`
- Version: `3`

## Layout

1. Header: magic, version, flags
2. Entry function name
3. State layout table
4. Function table + instruction streams

## State Layout Entry

- `name` (string)
- `type_name` (string)
- `has_default` (`u8`)
- `default_value` (encoded `Value`, when present)

## Value Encoding

- Tag `0`: `Int(i64)`
- Tag `1`: `Bool(u8)`
- Tag `2`: `String`
- Tag `3`: `Nil`
- Tag `4`: `Object(u64)`

## Added Instruction Families

In addition to arithmetic/control flow/call ops:

- Object heap ops: `AllocObject`, `GetProp`, `SetProp`, `Retain`, `Release`
- Native bridge op: `CallNative`
- Extended logic ops: `NotEquals`, `GreaterThan`, `LessOrEqual`, `GreaterOrEqual`, `Pop`
