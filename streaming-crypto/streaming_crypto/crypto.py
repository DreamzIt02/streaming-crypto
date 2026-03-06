# streaming_crypto/crypto.py

# This file imports the compiled PyO3 extension
# so Python users can access DigestAlg from streaming_crypto.crypto

from .crypto import DigestAlg

__all__ = ["DigestAlg"]

# from .streaming_crypto import crypto as _c

# DigestAlg = _c.DigestAlg

# __all__ = ["DigestAlg"]