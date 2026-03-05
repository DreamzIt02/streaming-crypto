from enum import IntFlag

class SegmentFlags(IntFlag):
    FINAL_SEGMENT: SegmentFlags
    COMPRESSED: SegmentFlags
    RESUMED: SegmentFlags
    RESERVED: SegmentFlags

class SegmentHeader:
    segment_index: int
    bytes_len: int
    wire_len: int
    wire_crc32: int
    frame_count: int
    digest_alg: int
    flags: SegmentFlags
    header_crc32: int

    LEN: int
    """
    FIXED Header LEN
    """

    @staticmethod
    def from_bytes(data: bytes) -> "SegmentHeader": ...
