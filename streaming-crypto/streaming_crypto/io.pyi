# streaming_crypto/io.pyi

from typing import Tuple
from .headers import HeaderV1

class PayloadReader:
    """
    Python wrapper for Rust `PayloadReader` that consumes the header.

    Usage:
        hdr, reader = PayloadReader.with_header(data: bytes)
    """

    @staticmethod
    def with_header(data: bytes) -> Tuple[HeaderV1, "PayloadReader"]: ...