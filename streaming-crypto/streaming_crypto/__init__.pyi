# streaming_crypto/__init__.pyi

from typing import Any, Dict, Optional, Union, IO
from dataclasses import dataclass

"""
High-performance streaming encryption library powered by Rust.

This package provides Python bindings for secure, efficient, and
scalable streaming encryption/decryption workflows. It exposes
Rust-backed primitives with Pythonic interfaces for ease of use.
"""

__all__ = [
    "encrypt",
    # Header and Params
    "PyHeaderV1",
    "PyEncryptParams",
    "PyDecryptParams",
    "PyApiConfig",
    # Telemetry
    "PyTelemetrySnapshot",
    # Error Types
    "PyCryptoError",
    # Streaming functions
    "py_encrypt_stream_v2",
    "py_decrypt_stream_v2",
]


def encrypt(data: bytes) -> bytes:
    """
    Encrypt raw bytes using a simple transformation.

    Args:
        data   : Input buffer containing plaintext bytes.

    Returns:
        bytes  : A new buffer containing encrypted bytes.

    Example:
        >>> encrypt(b"\x01\x02\x03")
        b'...'
    """
    ...


class PyEncryptParams:
    """
    Parameters required to configure an encryption operation.

    Attributes:
        header     : Metadata header describing algorithm and stream profile.
        dict       : Optional compression dictionary.
        master_key : Master key material used for encryption.
    """

    header: "PyHeaderV1"
    dict: Optional[bytes]
    master_key: bytes

    def __init__(self, master_key: bytes, header: "PyHeaderV1", dict: Optional[bytes]) -> None: ...


class PyDecryptParams:
    """
    Parameters required to configure a decryption operation.

    Attributes:
        master_key : Master key material used for decryption.
    """

    master_key: bytes

    def __init__(self, master_key: bytes) -> None: ...


class PyHeaderV1:
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

    magic: bytes
    version: int
    alg_profile: int
    cipher: int
    hkdf_prf: int
    compression: int
    strategy: int
    aad_domain: int
    flags: int
    chunk_size: int
    plaintext_size: int
    crc32: int
    dict_id: int
    salt: bytes
    key_id: int
    parallel_hint: int
    enc_time_ns: int
    reserved: bytes

    def __init__(
        self,
        magic: bytes,
        version: int,
        alg_profile: int,
        cipher: int,
        hkdf_prf: int,
        compression: int,
        strategy: int,
        aad_domain: int,
        flags: int,
        chunk_size: int,
        plaintext_size: int,
        crc32: int,
        dict_id: int,
        salt: bytes,
        key_id: int,
        parallel_hint: int,
        enc_time_ns: int,
        reserved: bytes,
    ) -> None: ...


class PyApiConfig:
    """
    API-level configuration options.

    Attributes:
        with_buf       : Whether to use buffered I/O.
        collect_metrics: Whether to collect telemetry metrics.
    """

    with_buf: Optional[bool]
    collect_metrics: Optional[bool]

    def __init__(
        self,
        with_buf: Optional[bool] = None,
        collect_metrics: Optional[bool] = None,
    ) -> None: ...


@dataclass
class PyTelemetrySnapshot:
    """
    Telemetry snapshot capturing performance and statistics
    from a streaming encryption/decryption operation.

    Attributes:
        segments_processed               : Number of segments processed.
        frames_data                      : Count of data frames.
        frames_terminator                : Count of terminator frames.
        frames_digest                    : Count of digest frames.
        bytes_plaintext                  : Total plaintext bytes processed.
        bytes_compressed                 : Total compressed bytes processed.
        bytes_ciphertext                 : Total ciphertext bytes produced.
        bytes_overhead                   : Overhead bytes added.
        compression_ratio                : Compression ratio achieved.
        throughput_plaintext_bytes_per_sec: Throughput in bytes/sec.
        elapsed_sec                      : Total elapsed time in seconds.
        stage_times                      : Mapping of stage names to elapsed times.
        output                           : Optional output buffer.
    """

    segments_processed: int
    frames_data: int
    frames_terminator: int
    frames_digest: int
    bytes_plaintext: int
    bytes_compressed: int
    bytes_ciphertext: int
    bytes_overhead: int
    compression_ratio: float
    throughput_plaintext_bytes_per_sec: float
    elapsed_sec: float
    stage_times: Dict[str, float]
    output: Optional[bytes]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def to_dict(self) -> Dict[str, Any]: ...


class PyCryptoError(Exception):
    """
    Exception type for cryptographic errors.

    Attributes:
        code    : Error code string.
        message : Human-readable error message.
    """

    code: str
    message: str

    def __init__(self, code: str, message: str) -> None: ...


def py_encrypt_stream_v2(
    input: Union[bytes, str, IO[bytes]],
    output: Union[bytes, str, IO[bytes]],
    params: PyEncryptParams,
    config: PyApiConfig,
) -> PyTelemetrySnapshot:
    """
    Perform streaming encryption on input data.

    Args:
        input  : Input source (bytes buffer, file path, or file-like object).
        output : Output target (bytes buffer, file path, or file-like object).
        params : Encryption parameters including header and key material.
        config : API configuration options.

    Returns:
        PyTelemetrySnapshot : Telemetry snapshot containing performance metrics and output data.
    """
    ...


def py_decrypt_stream_v2(
    input: Union[bytes, str, IO[bytes]],
    output: Union[bytes, str, IO[bytes]],
    params: PyDecryptParams,
    config: PyApiConfig,
) -> PyTelemetrySnapshot:
    """
    Perform streaming decryption on input data.

    Args:
        input  : Input source (bytes buffer, file path, or file-like object).
        output : Output target (bytes buffer, file path, or file-like object).
        params : Decryption parameters including key material.
        config : API configuration options.

    Returns:
        PyTelemetrySnapshot : Telemetry snapshot containing performance metrics and output data.
    """
    ...
