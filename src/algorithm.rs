fn permutation(n: u64, r: u64) -> u64 {
    (n - r + 1..=n).product()
}

fn combination(n: u64, mut r: u64) -> u64 {
    let perm = permutation(n, r);
    perm / (1..=r).product::<u64>()
}
