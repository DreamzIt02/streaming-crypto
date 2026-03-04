# streaming_crypto/__init__.pyi

from .headers import (Strategy, CipherSuite, HkdfPrf, AlgProfile, AadDomain, HeaderV1)
from .errors import CryptoError 
from .telemetry import TelemetrySnapshot
from .crypto import DigestAlg
from .parallelism import ParallelismConfig

from .api import (
     # Params
    EncryptParams,
    DecryptParams,
    ApiConfig,

    # Streaming functions
    encrypt_stream_v2,
    decrypt_stream_v2,
)

"""
High-performance streaming encryption library powered by Rust.

This package provides Python bindings for secure, efficient, and
scalable streaming encryption/decryption workflows. It exposes
Rust-backed primitives with Pythonic interfaces for ease of use.
"""

__all__ = [
    "encrypt",

    # Header and Types
    "Strategy",
    "CipherSuite",
    "HkdfPrf",
    "AlgProfile",
    "AadDomain",
    "HeaderV1",

    # Params
    "DigestAlg",
    "ParallelismConfig",
    "EncryptParams",
    "DecryptParams",
    "ApiConfig",

    # Telemetry
    "TelemetrySnapshot",
    # Error Types
    "CryptoError",

    # Streaming functions
    "encrypt_stream_v2",
    "decrypt_stream_v2",
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
