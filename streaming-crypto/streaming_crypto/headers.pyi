# streaming_crypto/headers.pyi

from enum import Enum

class CompressionCodec(Enum):
    Auto    : int
    Deflate : int
    Lz4     : int
    Zstd    : int

class Strategy(Enum):
    Auto        : int
    Sequential  : int
    Parallel    : int

class CipherSuite(Enum):
    Aes256Gcm       : int
    Chacha20Poly1305: int

class HkdfPrf(Enum):
    Sha256  : int
    Sha512  : int
    Sha3_256: int
    Sha3_512: int
    Blake3K : int

class AlgProfile(Enum):
    Aes256GcmHkdfSha256         : int
    Aes256GcmHkdfSha512         : int
    Chacha20Poly1305HkdfSha256  : int
    Chacha20Poly1305HkdfSha512  : int
    Chacha20Poly1305HkdfBlake3K : int

class AadDomain(Enum):
    Generic     : int
    FileEnvelope: int
    PipeEnvelope: int

class HeaderV1:
    """
    Structured header metadata for streaming encryption.

    This header defines algorithm selection, compression strategy,
    key identifiers, and other stream-level metadata.

    Attributes:
        magic         : 4-byte magic identifier.
        version       : Header version number.
        alg_profile   : Algorithm profile identifier.
        cipher        : Cipher suite identifier.
        hkdf_prf      : HKDF PRF identifier.
        compression   : Compression algorithm identifier.
        strategy      : Encryption strategy identifier.
        aad_domain    : Associated data domain identifier.
        flags         : Bit flags for stream options.
        chunk_size    : Size of each plaintext chunk.
        plaintext_size: Total plaintext size.
        crc32         : CRC32 checksum.
        dict_id       : Compression dictionary identifier.
        salt          : 16-byte salt.
        key_id        : Key identifier.
        parallel_hint : Hint for parallelism.
        enc_time_ns   : Encryption timestamp (nanoseconds).
        reserved      : 8-byte reserved field.
    """

    magic       : bytes
    version     : int
    alg_profile : int
    cipher      : int
    hkdf_prf    : int
    compression : int
    strategy    : int
    aad_domain  : int
    flags       : int
    chunk_size  : int
    plaintext_size: int
    crc32       : int
    dict_id     : int
    salt        : bytes
    key_id      : int
    parallel_hint: int
    enc_time_ns : int
    reserved    : bytes

    def __init__(
        self,
        magic       : bytes,
        version     : int,
        alg_profile : int,
        cipher      : int,
        hkdf_prf    : int,
        compression : int,
        strategy    : int,
        aad_domain  : int,
        flags       : int,
        chunk_size  : int,
        plaintext_size: int,
        crc32       : int,
        dict_id     : int,
        salt        : bytes,
        key_id      : int,
        parallel_hint: int,
        enc_time_ns : int,
        reserved    : bytes,
    ) -> None: ...

    LEN: int
    """
    FIXED Header LEN
    """

__all__ = [
    # Header and Types
    "CompressionCodec",
    "Strategy",
    "CipherSuite",
    "HkdfPrf",
    "AlgProfile",
    "AadDomain",
    "HeaderV1",
]