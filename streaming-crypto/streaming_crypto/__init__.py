# streaming_crypto/__init__.py
import io
from pathlib import Path
from typing import Union, IO

# This must be imported from .streaming_crypto, otherwise
# ImportError: cannot import name 'encrypt' from partially initialized module 'streaming_crypto' (most likely due to a circular import)
from .streaming_crypto import (
    encrypt,

    # Params
    EncryptParams,
    DecryptParams,
    ApiConfig,

    # Telemetry
    TelemetrySnapshot,

    # Error Types
    StreamError,

    # Streaming functions
    encrypt_stream_v2,
    decrypt_stream_v2,
)

class InputSource:
    """
    Convenience wrappers for specifying the input to encrypt/decrypt pipelines.

    These are pass-through helpers — they return the underlying Python object
    unchanged. The Rust pipeline inspects the object type at runtime via
    classify_py_io() and selects the optimal I/O path automatically.

    No special objects are created. Rust never sees InputSource — only the
    unwrapped bytes, str, or file-like object.
    """

    @staticmethod
    def Memory(data: bytes) -> bytes:
        """
        Read input from an in-memory bytes buffer.

        Zero-copy: the Rust pipeline borrows directly into Python's immutable
        bytes buffer without copying. The GIL is released for the entire
        crypto pipeline while the borrow is held.

        Args:
            data: Input plaintext (encrypt) or ciphertext (decrypt) as bytes.
                  Must be immutable bytes, not bytearray. For bytearray,
                  the pipeline will copy the buffer for safety (mutable
                  buffers can be resized by Python while Rust holds a reference).

        Returns:
            The same bytes object, passed through unchanged.

        Example:
            encrypt_stream_v2(input=InputSource.Memory(plaintext), ...)
        """
        return data

    @staticmethod
    def File(path: Union[str, Path]) -> str:
        """
        Read input from a file path.

        Zero-copy: the Rust pipeline opens and reads the file directly via
        the OS, bypassing Python's I/O layer entirely. No data passes through
        Python memory.

        Args:
            path: Absolute or relative path to the input file, as str or Path.

        Returns:
            The path as-is (str or Path), passed through unchanged.

        Example:
            encrypt_stream_v2(input=InputSource.File("/data/plain.bin"), ...)
        """
        return str(path)

    @staticmethod
    def Reader(file_obj: Union[IO[bytes], io.BytesIO]) -> Union[IO[bytes], io.BytesIO]:
        """
        Read input from any file-like object supporting .read().

        The Rust pipeline calls .read() on the object in chunks, streaming
        data through without buffering the full input in memory. Compatible
        with BytesIO, open() handles, network sockets, and any object
        implementing the read protocol.

        Args:
            file_obj: Any readable binary file-like object. Must support
                      .read(n) -> bytes. The GIL is re-acquired for each
                      .read() call since Python objects are involved.

        Returns:
            The file-like object, passed through unchanged.

        Example:
            with open("/data/plain.bin", "rb") as f:
                encrypt_stream_v2(input=InputSource.Reader(f), ...)
        """
        return file_obj


class OutputSink:
    """
    Convenience wrappers for specifying the output destination of
    encrypt/decrypt pipelines.

    These are pass-through helpers — they return the underlying Python object
    unchanged (or a sentinel for Memory). The Rust pipeline inspects the
    object type at runtime via classify_py_io() and selects the optimal
    I/O path automatically.

    For large data, prefer File() or Writer() over Memory() to avoid the
    one unavoidable copy at the Rust→Python boundary.
    """

    @staticmethod
    def Memory() -> bytes:
        """
        Capture output in memory, returned via TelemetrySnapshot.output.

        Returns an empty bytes sentinel (b""). The Rust pipeline detects
        this and writes output into an internal buffer, which is then
        transferred to Python as a PyBytes object.

        Copy cost: exactly one copy occurs at the Rust→Python boundary
        (Rust Vec<u8> → Python PyBytes). This is unavoidable — Python's
        memory manager must own its buffer. For large payloads, prefer
        File() or Writer() to eliminate this copy entirely.

        Returns:
            b"" sentinel — do not use the return value directly.
            Access the output via TelemetrySnapshot.output after the call.

        Example:
            snapshot = encrypt_stream_v2(
                input=InputSource.Memory(plaintext),
                output=OutputSink.Memory(),
                ...
            )
            ciphertext = snapshot.output  # PyBytes, one copy from Rust heap
        """
        return b""

    @staticmethod
    def File(path: Union[str, Path]) -> str:
        """
        Write output directly to a file path.

        Zero-copy: the Rust pipeline writes encoded segments directly to
        the file descriptor via the OS, bypassing Python's I/O layer.
        No output data passes through Python memory at any point.

        Args:
            path: Absolute or relative path to the output file, as str or Path.
                  The file will be created or overwritten.

        Returns:
            The path as-is (str or Path), passed through unchanged.

        Example:
            encrypt_stream_v2(output=OutputSink.File("/data/out.enc"), ...)
        """
        return str(path)

    @staticmethod
    def Writer(file_obj: Union[IO[bytes], io.BytesIO]) -> Union[IO[bytes], io.BytesIO]:
        """
        Write output to any file-like object supporting .write().

        Zero-copy: the Rust pipeline calls .write() directly on the object
        as each segment completes, streaming output without accumulating
        it in memory. The reorder buffer holds at most ~2-3 segments
        transiently before draining to the writer.

        Compatible with BytesIO, open() handles, network sockets, and any
        object implementing the write protocol.

        Args:
            file_obj: Any writable binary file-like object. Must support
                      .write(bytes). The GIL is re-acquired for each
                      .write() call since Python objects are involved.

        Returns:
            The file-like object, passed through unchanged.

        Example:
            buf = io.BytesIO()
            encrypt_stream_v2(output=OutputSink.Writer(buf), ...)
            ciphertext = buf.getvalue()
        """
        return file_obj

# Optional: define __all__ for clean autocompletion
__all__ = [
    "encrypt",

    # Params
    "InputSource", 
    "OutputSink", 
    "EncryptParams",
    "DecryptParams",
    "ApiConfig",
    # Telemetry
    "TelemetrySnapshot",
    # Error Types
    "StreamError",

    # Streaming functions
    "encrypt_stream_v2",
    "decrypt_stream_v2",
]
