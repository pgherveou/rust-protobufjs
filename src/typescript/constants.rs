use phf::phf_map;

/// A list of proto type to Typescript type
pub static TYPE_MAPPING: phf::Map<&'static str, &'static str> = phf_map! {
    ".google.protobuf.StringValue" => "string",
    ".google.protobuf.BoolValue" => "boolean",
    ".google.protobuf.BytesValue" => "Buffer",
    ".google.protobuf.Int32Value" => "number",
    ".google.protobuf.UInt32Value" => "number",
    ".google.protobuf.Int64Value" => "LongLike",
    ".google.protobuf.UInt64Value" => "LongLike",
    ".google.protobuf.FloatValue" => "number",
    ".google.protobuf.DoubleValue" => "number",
    ".google.protobuf.Timestamp" => "globalThis.Date | string",
    ".google.protobuf.Duration" => "string",
    "float" => "number",
    "bool" => "boolean",
    "uint64" => "LongLike",
    "fixed64" => "LongLike",
    "int64" => "LongLike",
    "sint64" => "LongLike",
    "int32" => "number",
    "sfixed32" => "number",
    "sint32" => "number",
    "uint32" => "number",
    "double" => "number",
    "string" => "string",
    "bytes" => "Buffer",
};

// Extra types imported

pub const OBSERVABLE_IMPORT: &str = "import { Observable } from 'rxjs'";
pub const BUBBLE_CLIENT_IMPORT: &str = "import { RouteHandler } from '@lyft/bubble-client'";
pub const NETWORK_CLIENT_IMPORT: &str =
    "import { GRPCResource, HTTPResource } from '@lyft/network-client'";

pub const LONG_LIKE_TYPE: &str = r#"  
  type LongLike = number | BigInt | { toNumber(): number }"#;

pub const ANY_TYPE: &str = r#"  
  type AnyType<T = Record<string, unknown>> = T & {
    // reference to the type serialized (e.g 'pb.api.endpoints.v1.core_trips.GetActiveTripsResponse')
    '@type': string
  }"#;

pub const EMPTY: &str = r#"  
  interface Empty { _?: never }"#;
