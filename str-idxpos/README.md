# str-idxpos

A library for converting 0-based string indexes to 0-based line/column positions and back.

```rust
use str_idxpos::{index_to_position, position_to_index};

fn main () {
    let input = "Hello\nWorld!";
    let index = 8usize;

    let position = index_to_position(input, index);

    assert_eq!((1, 1), position);
    assert_eq!(index, position_to_index(input, position));
}
```
