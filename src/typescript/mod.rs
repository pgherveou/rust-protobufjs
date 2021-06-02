//! Generate Typescript definitions from parsed proto namespace
//!
//! # Example:
//! Given the following proto file
//!
//! ```proto
//! package pb.hello;
//!
//! service HelloWorld {
//!   rpc LotsOfGreetings(stream SayHelloRequest) returns (SayHelloResponses) {}
//! }
//!
//! message SayHelloRequest {
//!   string name = 1;
//! }
//!
//! message SayHelloResponse {
//!   string hello = 1;
//! }
//!
//! message SayHelloResponses {
//!   repeated SayHelloResponse responses = 1;
//! }
//! ```
//! This module can generate the following Typescript definition:
//!
//! ```ts
//! import { Observable } from 'rxjs'
//! import { RouteHandler } from '@lyft/bubble-client'
//! import { GRPCResource, HTTPResource } from '@lyft/network-client'
//!
//! declare module '@lyft/bubble-client' {
//!   interface Router {
//!     /**
//!      * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#6
//!      */
//!     grpc(path: '/pb.hello/LotsOfGreetings', handler: RouteHandler<Observable<pb.hello.SayHelloRequest>, pb.hello.SayHelloResponses, [code: number, body: string]>): void
//!
//!     /**
//!      * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#7
//!      */
//!     get(path: '/hello/<string:name>', handler: RouteHandler<pb.hello.SayHelloRequest, pb.hello.SayHelloResponse, [code: number, body: unknown]>): void
//!   }
//! }
//!
//! declare module '@lyft/network-client' {
//!   interface NetworkClient {
//!     /**
//!      * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#6
//!      */
//!     grpc(path: '/pb.hello/LotsOfGreetings', handler: GRPCResource<Observable<pb.hello.SayHelloRequest>, pb.hello.SayHelloResponses, [code: number, body: string]>): void
//!
//!     /**
//!      * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#7
//!      */
//!     get(path: '/hello/<string:name>'): HTTPResource<pb.hello.SayHelloRequest, pb.hello.SayHelloResponse>
//!   }
//! }
//!
//! declare global {
//!   namespace pb {
//!     namespace hello {
//!       /**
//!        * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#12
//!        */
//!       interface SayHelloRequest {
//!         name?: string
//!       }
//!
//!       /**
//!        * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#16
//!        */
//!       interface SayHelloResponse {
//!         hello?: string
//!       }
//!
//!       /**
//!        * @link https://github.com/lyft/idl/blob/master/protos/pb/lyft/hello/hello_world.proto#20
//!        */
//!       interface SayHelloResponses {
//!         responses?: Array<pb.hello.SayHelloResponse>
//!       }
//!     }
//!   }
//! }

mod constants;
pub mod serializer;
