# streaming_crypto/errors.pyi

class StreamError(Exception):
    """
    Exception type for cryptographic errors.

    Attributes:
        code    : Error code string
        message : Human-readable error message
    """

    code: str
    message: str

    def __init__(self, code: str, message: str) -> None: ...
