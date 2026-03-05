from enum import IntEnum

class FrameType(IntEnum):
    Data: int
    Terminator: int
    Digest: int

class FrameHeader:
    LEN: int

    segment_index: int
    frame_index: int
    frame_type: FrameType
    plaintext_len: int
    ciphertext_len: int

    def __repr__(self) -> str: ...
