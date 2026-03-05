# streaming_crypto/parallelism.py

# This file imports the compiled PyO3 extension
# so Python users can access ParallelismConfig from streaming_crypto.parallelism

from .parallelism import ParallelismConfig

__all__ = ["ParallelismConfig"]
