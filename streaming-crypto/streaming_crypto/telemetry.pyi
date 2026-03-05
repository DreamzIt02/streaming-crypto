# streaming_crypto/telemetry.pyi

from dataclasses import dataclass
from typing import Any, Dict, Optional

@dataclass
class TelemetrySnapshot:
    """
    Telemetry snapshot capturing performance and statistics
    from a streaming encryption/decryption operation.

    Attributes:
        segments_processed               : Number of segments processed.
        frames_data                      : Count of data frames.
        frames_terminator                : Count of terminator frames.
        frames_digest                    : Count of digest frames.
        bytes_plaintext                  : Total plaintext bytes processed.
        bytes_compressed                 : Total compressed bytes processed.
        bytes_ciphertext                 : Total ciphertext bytes produced.
        bytes_overhead                   : Overhead bytes added.
        compression_ratio                : Compression ratio achieved.
        throughput_plaintext_bytes_per_sec: Throughput in bytes/sec.
        elapsed_sec                      : Total elapsed time in seconds.
        stage_times                      : Mapping of stage names to elapsed times.
        output                           : Optional output buffer.
    """

    segments_processed: int
    frames_data: int
    frames_terminator: int
    frames_digest: int
    bytes_plaintext: int
    bytes_compressed: int
    bytes_ciphertext: int
    bytes_overhead: int
    compression_ratio: float
    throughput_plaintext_bytes_per_sec: float
    elapsed_sec: float
    stage_times: Dict[str, float]
    output: Optional[bytes]

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def to_dict(self) -> Dict[str, Any]: ...
