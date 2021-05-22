use phf::phf_set;

/// scalars defines all the possible [scalar value types]
/// [scalar value types] https://developers.google.com/protocol-buffers/docs/overview#scalar
pub static SCALARS: phf::Set<&'static str> = phf_set! {
    "double", "float",
    "int32", "int64", "uint32", "uint64", "sint32", "sint64",
    "fixed32", "fixed64", "sfixed32", "sfixed64",
    "bool", "string", "bytes"
};
