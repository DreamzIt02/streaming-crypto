# streaming_crypto/api.pyi

from typing import IO, Optional, Union
from .headers import (HeaderV1)
from .telemetry import TelemetrySnapshot

__all__ = [
    # Params
    "EncryptParams",
    "DecryptParams",
    "ApiConfig",

    # Streaming functions
    "encrypt_stream_v2",
    "decrypt_stream_v2",
]

class EncryptParams:
    """
    Parameters required to configure an encryption operation.

    Attributes:
        master_key : Master key material used for encryption.
        header     : Metadata header describing algorithm and stream profile.
        dict       : Optional compression dictionary.
    """

    master_key: bytes
    header: "HeaderV1"
    dict: Optional[bytes]

    def __init__(self, master_key: bytes, header: "HeaderV1", dict: Optional[bytes]) -> None: ...


class DecryptParams:
    """
    Parameters required to configure a decryption operation.

    Attributes:
        master_key : Master key material used for decryption.
    """

    master_key: bytes

    def __init__(self, master_key: bytes) -> None: ...


class ApiConfig:
    """
    API-level configuration options.

    Attributes:
        with_buf       : Whether to use buffered I/O.
        collect_metrics: Whether to collect telemetry metrics.
    """

    with_buf: Optional[bool]
    collect_metrics: Optional[bool]

    def __init__(
        self,
        with_buf: Optional[bool] = None,
        collect_metrics: Optional[bool] = None,
    ) -> None: ...

def encrypt_stream_v2(
    input: Union[bytes, str, IO[bytes]],
    output: Union[bytes, str, IO[bytes]],
    params: EncryptParams,
    config: ApiConfig,
) -> TelemetrySnapshot:
    """
    Perform streaming encryption on input data.

    Args:
        input  : Input source (bytes buffer, file path, or file-like object).
        output : Output target (bytes buffer, file path, or file-like object).
        params : Encryption parameters including header and key material.
        config : API configuration options.

    Returns:
        TelemetrySnapshot : Telemetry snapshot containing performance metrics and output data.
    """
    ...


def decrypt_stream_v2(
    input: Union[bytes, str, IO[bytes]],
    output: Union[bytes, str, IO[bytes]],
    params: DecryptParams,
    config: ApiConfig,
) -> TelemetrySnapshot:
    """
    Perform streaming decryption on input data.

    Args:
        input  : Input source (bytes buffer, file path, or file-like object).
        output : Output target (bytes buffer, file path, or file-like object).
        params : Decryption parameters including key material.
        config : API configuration options.

    Returns:
        TelemetrySnapshot : Telemetry snapshot containing performance metrics and output data.
    """
    ...
