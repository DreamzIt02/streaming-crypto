# streaming_crypto/api.py

# This must be imported from .streaming_crypto, otherwise
# ImportError: cannot import name 'encrypt' from partially initialized module 'streaming_crypto' (most likely due to a circular import)
from .streaming_crypto.api import (
    # Params
    EncryptParams,
    DecryptParams,
    ApiConfig,

    # Streaming functions
    encrypt_stream_v2,
    decrypt_stream_v2,
)

# Optional: define __all__ for clean autocompletion
__all__ = [
    # Params
    "EncryptParams",
    "DecryptParams",
    "ApiConfig",

    # Streaming functions
    "encrypt_stream_v2",
    "decrypt_stream_v2",
]
