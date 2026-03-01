# streaming_crypto/__init__.pyi

def encrypt(data: bytes) -> bytes: ...
"""
Encrypts data by XORing each byte with 0xAA.

# Examples

```
use streaming_crypto::encrypt;

let data = vec![1, 2, 3];
let encrypted = encrypt(&data);
assert_eq!(encrypted[0], 1 ^ 0xAA);
assert_eq!(encrypted[1], 2 ^ 0xAA);
assert_eq!(encrypted[2], 3 ^ 0xAA);
"""

__all__ = ["encrypt"]