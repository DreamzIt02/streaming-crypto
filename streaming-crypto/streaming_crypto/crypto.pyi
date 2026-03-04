# streaming_crypto/crypto.pyi

from enum import Enum

class DigestAlg(Enum):
    Sha256: int
    Sha512: int
    Sha3_256: int
    Sha3_512: int
    Blake3: int