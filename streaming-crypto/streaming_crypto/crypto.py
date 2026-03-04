# streaming_crypto/crypto.py

# This file imports the compiled PyO3 extension
# so Python users can access DigestAlg from streaming_crypto.crypto

from .streaming_crypto.crypto import DigestAlg

__all__ = ["DigestAlg"]
