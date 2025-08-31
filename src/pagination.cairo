use core::array::Array;
use core::array::ArrayTrait;

/// Safely slices an array with offset and limit, handling bounds
pub fn safe_slice<T, impl TDrop: core::traits::Drop<T>, impl TCopy: core::traits::Copy<T>>(
    arr: Array<T>, 
    offset: u32, 
    limit: u32
) -> Array<T> {
    let len = arr.len();
    
    // If offset is beyond array length, return empty array
    if offset >= len {
        return array![];
    }
    
    // Calculate the end index
    let end = if offset + limit > len { len } else { offset + limit };
    
    // Extract the slice
    let mut result = array![];
    let mut i = offset;
    loop {
        if i >= end {
            break;
        };
        result.append(*arr.at(i));
        i += 1;
    };
    
    result
}

/// Validates pagination parameters
pub fn validate_pagination(offset: u32, limit: u32) {
    assert(limit > 0, 'ERR_INVALID_LIMIT');
    assert(limit <= 100, 'ERR_LIMIT_TOO_LARGE');
}
