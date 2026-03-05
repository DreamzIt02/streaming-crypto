# streaming_crypto/__init__.py

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

# Optional: define __all__ for clean autocompletion
__all__ = [
    "encrypt",

    # Params
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
