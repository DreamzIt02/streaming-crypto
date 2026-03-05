# streaming_crypto/io.py

from .streaming_crypto.io import PayloadReader as _RustPayloadReader
from .headers import HeaderV1

__all__ = ["PayloadReader"]

class PayloadReader:
    """
    Python wrapper around Rust PayloadReader.

    Provides `.with_header()` to parse header and return reader.
    """

    @staticmethod
    def with_header(data: bytes) -> tuple[HeaderV1, "_RustPayloadReader"]:
        """
        Parse the header from the given bytes and return a tuple:

            header, reader = PayloadReader.with_header(data)

        Raises StreamError on invalid data.
        """
        return _RustPayloadReader.with_header(data)