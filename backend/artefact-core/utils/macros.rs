macro_rules! mul_add {
    ($a:expr, $b:expr, $c:expr) => {
        $a.mul_add($b, $c)
    };
}

pub(crate) use mul_add;
