# streaming_crypto/parallelism.pyi

from typing import Dict

class ParallelismConfig:
    cpu_workers: int
    gpu_workers: int
    mem_fraction: float
    hard_cap: int

    def __init__(self, cpu_workers: int, gpu_workers: int, mem_fraction: float, hard_cap: int) -> None: ...
    def as_dict(self) -> Dict[str, object]: ...

__all__ = ["ParallelismConfig"]
