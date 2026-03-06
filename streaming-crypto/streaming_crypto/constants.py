# streaming_crypto/constants.py

from .constants import (
    MAGIC_RSE1,
    HEADER_V1,
    DEFAULT_CHUNK_SIZE,
    MAX_CHUNK_SIZE,
    flags,
)

__all__ = [
    "MAGIC_RSE1",
    "HEADER_V1",
    "DEFAULT_CHUNK_SIZE",
    "MAX_CHUNK_SIZE",
    "flags",
]

# from .streaming_crypto import constants as _c

# MAGIC_RSE1          = _c.MAGIC_RSE1
# HEADER_V1           = _c.HEADER_V1
# DEFAULT_CHUNK_SIZE  = _c.DEFAULT_CHUNK_SIZE
# MAX_CHUNK_SIZE      = _c.MAX_CHUNK_SIZE
# flags               = _c.flags

# __all__ = ["MAGIC_RSE1", "HEADER_V1", "DEFAULT_CHUNK_SIZE", "MAX_CHUNK_SIZE", "flags"]