

fn npr(mut n: i32,mut r: i32) -> i32{
    let mut number = 1;
    while r > 0{
        number *= r;
        n -= 1;
        r -= 1;
    }
    number
}

fn ncr(n: i32,mut r: i32) -> i32{
    let perm = npr(n,r);
    let mut number = 1;
    while r > 0{
        number *= r;
        r -= 1;
    }
    perm/number
}