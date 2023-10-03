

pub fn copy<const N: usize>(slice: &[u8]) -> [u8; N] {
    let mut array = [0u8; N];
    array.copy_from_slice(&slice[..N]);
    array
}
