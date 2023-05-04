//! This module provides the utilities needed to easily implement a Tokio
//! transport using [serde] for serialization and deserialization of frame
//! values.
//!
//! # Introduction
//!
//! This crate provides [transport] combinators that transform a stream of
//! frames encoded as bytes into a stream of frame values. It is expected that
//! the framing happens at another layer. One option is to use a [length
//! delimited] framing transport.
//!
//! The crate provides two traits that must be implemented: [`Serializer`] and
//! [`Deserializer`]. Implementations of these traits are then passed to
//! [`Framed`] along with the upstream [`Stream`] or
//! [`Sink`] that handles the byte encoded frames.
//!
//! By doing this, a transformation pipeline is built. For reading, it looks
//! something like this:
//!
//! * `manic_crypto::Framed`
//! * `tokio_util::codec::FramedRead`
//! * `tokio::net::TcpStream`
//!
//! The write half looks like:
//!
//! * `manic_crypto::Framed`
//! * `tokio_util::codec::FramedWrite`
//! * `tokio::net::TcpStream`
//!
//! [serde]: https://serde.rs
//! [serde-json]: https://github.com/serde-rs/json
//! [transport]: https://tokio.rs/docs/going-deeper/transports/
//! [length delimited]: https://docs.rs/tokio-util/0.2/tokio_util/codec/length_delimited/index.html
//! [`Serializer`]: trait.Serializer.html
//! [`Deserializer`]: trait.Deserializer.html
//! [`Framed`]: struct.Framed.html
//! [`Stream`]: https://docs.rs/futures/0.3/futures/stream/trait.Stream.html
//! [`Sink`]: https://docs.rs/futures/0.3/futures/sink/trait.Sink.html
use crate::codecs::{Crypter, Packet};
use aead::{Aead, AeadCore};
use buildstructor::buildstructor;
use bytes::{Bytes, BytesMut};
use futures::{ready, Sink, Stream, TryStream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{
    Decoder, Framed, FramedParts, FramedRead, FramedWrite, LengthDelimitedCodec,
};

/// Serializes a value into a destination buffer
///
/// Implementations of `Serializer` are able to take values of type `T` and
/// convert them to a byte representation. The specific byte format, i.e. JSON,
/// protobuf, binpack, ... is an implementation detail.
///
/// The `serialize` function takes `&mut self`, allowing for `Serializer`
/// instances to be created with runtime configuration settings.
///
/// # Examples
///
/// An integer serializer that allows the width to be configured.
///
/// ```
/// use tokio_serde::Serializer;
/// use bytes::{Buf, Bytes, BytesMut, BufMut};
/// use std::pin::Pin;
///
/// struct IntSerializer {
///     width: usize,
/// }
///
/// #[derive(Debug)]
/// enum Error {
///     Overflow,
/// }
///
/// impl Serializer<u64> for IntSerializer {
///     type Error = Error;
///
///     fn serialize(self: Pin<&mut Self>, item: &u64) -> Result<Bytes, Self::Error> {
///         assert!(self.width <= 8);
///
///         let max = (1 << (self.width * 8)) - 1;
///
///         if *item > max {
///             return Err(Error::Overflow);
///         }
///
///         let mut ret = BytesMut::with_capacity(self.width);
///         ret.put_uint(*item, self.width);
///         Ok(ret.into())
///     }
/// }
///
/// let mut serializer = IntSerializer { width: 3 };
///
/// let buf = Pin::new(&mut serializer).serialize(&5).unwrap();
/// assert_eq!(buf, &b"\x00\x00\x05"[..]);
/// ```
pub trait Serializer {
    type Error;

    /// Serializes `item` into a new buffer
    ///
    /// The serialization format is specific to the various implementations of
    /// `Serializer`. If the serialization is successful, a buffer containing
    /// the serialized item is returned. If the serialization is unsuccessful,
    /// an error is returned.
    ///
    /// Implementations of this function should not mutate `item` via any sort
    /// of internal mutability strategy.
    ///
    /// See the trait level docs for more detail.
    fn serialize(self: Pin<&mut Self>, item: &Packet) -> Result<Bytes, Self::Error>;
}

/// Deserializes a value from a source buffer
///
/// Implementatinos of `Deserializer` take a byte buffer and return a value by
/// parsing the contents of the buffer according to the implementation's format.
/// The specific byte format, i.e. JSON, protobuf, binpack, is an implementation
/// detail
///
/// The `deserialize` function takes `&mut self`, allowing for `Deserializer`
/// instances to be created with runtime configuration settings.
///
/// It is expected that the supplied buffer represents a full value and only
/// that value. If after deserializing a value there are remaining bytes the
/// buffer, the deserializer will return an error.
///
/// # Examples
///
/// An integer deserializer that allows the width to be configured.
///
/// ```
/// use tokio_serde::Deserializer;
/// use bytes::{BytesMut, Buf};
/// use std::pin::Pin;
///
/// struct IntDeserializer {
///     width: usize,
/// }
///
/// #[derive(Debug)]
/// enum Error {
///     Underflow,
///     Overflow
/// }
///
/// impl Deserializer<u64> for IntDeserializer {
///     type Error = Error;
///
///     fn deserialize(self: Pin<&mut Self>, buf: &BytesMut) -> Result<u64, Self::Error> {
///         assert!(self.width <= 8);
///
///         if buf.len() > self.width {
///             return Err(Error::Overflow);
///         }
///
///         if buf.len() < self.width {
///             return Err(Error::Underflow);
///         }
///
///         let ret = std::io::Cursor::new(buf).get_uint(self.width);
///         Ok(ret)
///     }
/// }
///
/// let mut deserializer = IntDeserializer { width: 3 };
///
/// let i = Pin::new(&mut deserializer).deserialize(&b"\x00\x00\x05"[..].into()).unwrap();
/// assert_eq!(i, 5);
/// ```
pub trait Deserializer {
    type Error;

    /// Deserializes a value from `buf`
    ///
    /// The serialization format is specific to the various implementations of
    /// `Deserializer`. If the deserialization is successful, the value is
    /// returned. If the deserialization is unsuccessful, an error is returned.
    ///
    /// See the trait level docs for more detail.
    fn deserialize(self: Pin<&mut Self>, src: &BytesMut) -> Result<Packet, Self::Error>;
}

/// Adapts a transport to a value sink by serializing the values and to a stream of values by deserializing them.
///
/// It is expected that the buffers yielded by the supplied transport be framed. In
/// other words, each yielded buffer must represent exactly one serialized
/// value.
///
/// The provided transport will receive buffer values containing the
/// serialized value. Each buffer contains exactly one value. This sink will be
/// responsible for writing these buffers to an `AsyncWrite` using some sort of
/// framing strategy.
///
/// The specific framing strategy is left up to the
/// implementor. One option would be to use [length_delimited] provided by
/// [tokio-util].
///
/// [length_delimited]: http://docs.rs/tokio-util/0.2/tokio_util/codec/length_delimited/index.html
/// [tokio-util]: http://crates.io/crates/tokio-util
#[pin_project]
#[derive(Debug)]
pub struct CryptoFramed<T, C, A>
where
    T: AsyncRead + AsyncWrite,
    C: Codec,
    A: Aead + AeadCore,
{
    #[pin]
    inner: Framed<T, LengthDelimitedCodec>,
    #[pin]
    codec: Crypter<C, A>,
}

#[buildstructor]
impl<T, C, A> CryptoFramed<T, C, A> {
    /// Creates a new `Framed` with the given transport and codec.
    #[builder]
    pub fn new(inner: T, inner_codec: C, crypt: A) -> Self {
        let inner = LengthDelimitedCodec::new().framed(inner);

        Self { inner, codec }
    }

    /// Returns a reference to the underlying transport wrapped by `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying transport as
    /// it may corrupt the sequence of frames otherwise being worked with.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the underlying transport wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying transport as
    /// it may corrupt the sequence of frames otherwise being worked with.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the `Framed`, returning its underlying transport.
    ///
    /// Note that care should be taken to not tamper with the underlying transport as
    /// it may corrupt the sequence of frames otherwise being worked with.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<Transport, Codec> Stream for CryptoFramed<Transport, Codec>
where
    Transport: TryStream<Ok = BytesMut>,
    Transport::Error: From<Codec::Error>,
    BytesMut: From<Transport::Ok>,
    Codec: Deserializer,
{
    type Item = Result<Packet, Transport::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.as_mut().project().inner.try_poll_next(cx)) {
            Some(bytes) => Poll::Ready(Some(Ok(self
                .as_mut()
                .project()
                .codec
                .deserialize(&bytes?)?))),
            None => Poll::Ready(None),
        }
    }
}

impl<Transport, Codec> Sink<Packet> for CryptoFramed<Transport, Codec>
where
    Transport: Sink<Bytes>,
    Codec: Serializer,
    Codec::Error: Into<Transport::Error>,
{
    type Error = Transport::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        let res = self.as_mut().project().codec.serialize(&item);
        let bytes = res.map_err(Into::into)?;

        self.as_mut().project().inner.start_send(bytes)?;

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        self.project().inner.poll_close(cx)
    }
}
