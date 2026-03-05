# streaming_crypto/constants.pyi

from typing import Final

# Protocol magic/version
MAGIC_RSE1:         Final[bytes]        # b"RSE1"
HEADER_V1:          Final[int]          # 1

# Chunk sizes
DEFAULT_CHUNK_SIZE: Final[int]          # 65536
MAX_CHUNK_SIZE:     Final[int]          # 33554432

# Flags subgroup (submodule)
class flags:
    HAS_TOTAL_LEN:  Final[int]          # 0x0001
    HAS_CRC32:      Final[int]          # 0x0002
    HAS_TERMINATOR: Final[int]          # 0x0004
    HAS_FINAL_DIGEST: Final[int]        # 0x0008
    DICT_USED:      Final[int]          # 0x0010
    AAD_STRICT:     Final[int]          # 0x0020

__all__ = [
    "MAGIC_RSE1",
    "HEADER_V1",
    "DEFAULT_CHUNK_SIZE",
    "MAX_CHUNK_SIZE",
    "flags",
]
