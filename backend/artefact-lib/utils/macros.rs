#[cfg(feature = "native")]
macro_rules! mul_add {
    ($a:expr, $b:expr, $c:expr) => {
        $a.mul_add($b, $c)
    };
}

#[cfg(not(feature = "native"))]
macro_rules! mul_add {
    ($a:expr, $b:expr, $c:expr) => {
        $a * $b + $c
    };
}

pub(crate) use mul_add;
