use geng::prelude::*;

pub fn smoothstep<T: Float>(t: T) -> T {
    let two = T::ONE + T::ONE;
    let three = two + T::ONE;
    three * t * t - two * t * t * t
}
