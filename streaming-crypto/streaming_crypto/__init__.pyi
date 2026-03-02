# streaming_crypto/__init__.pyi

"""
High-performance streaming encryption library powered by Rust.
"""

def encrypt(data: bytes) -> bytes:
    """
    Encrypts data by XORing each byte with 0xAA.

    Example:
        >>> encrypt(b"\x01\x02\x03")
        b'\xab\xa8\xa9'
    """
    ...

__all__ = ["encrypt"]