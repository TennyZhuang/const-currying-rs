[![Latest Version]][crates.io]
[![Documentation]][docs.rs]

[Latest Version]: https://img.shields.io/crates/v/const-currying.svg
[crates.io]: https://crates.io/crates/const-currying
[Documentation]: https://img.shields.io/docsrs/const-currying
[docs.rs]: https://docs.rs/const-currying

# Const-currying

A rust proc macro that help you improve your function's performance using [curring techniques](https://en.wikipedia.org/wiki/Currying).

## Motivation

Const generics is a feature of rust that allows you to pass const values as generic parameters. This feature is very useful to improve the performance of your code, by generating multiple versions of the same function at compile time. However, sometimes you have to call the function with a runtime-dependent value, even if it's likely a known const value, or a part of the calls are with const values. This macro helps you to generate multiple versions of the same function, with either const or runtime values, at compile time.

There is an excited crate [partial_const](https://crates.io/crates/partial_const), which already provides a similar feature. It's fully based on the wonderful type system of rust, however, it requires caller to specify the const value explicitly. Our crate provides a proc-macro based solution, which introduces no invasive changes to the caller's code.

## Usage

You can specify the potential constant values of the function's arguments using `maybe_const` attribute.

```rust
use const_currying::const_currying;

#[const_currying]
fn f1(
    #[maybe_const(dispatch = x, consts = [0, 1])] x: i32,
    #[maybe_const(dispatch = y, consts = [true, false])] y: bool,
    z: &str,
) -> i32 {
    if y {
        x
    } else {
        -x
    }
}
```

There are two arguments `x` and `y` which are potentially passed as const values. The optional `dispatch` attribute specifies the suffix of the generated function name. As an example, we can see the full generated codes here.

```rust
#[allow(warnings)]
fn f1_orig(x: i32, y: bool, z: &str) -> (i32, String) {
    if y { (x, z.to_string()) } else { (-x, z.chars().rev().collect()) }
}
#[allow(warnings)]
fn f1_x<const x: i32>(y: bool, z: &str) -> (i32, String) {
    if y { (x, z.to_string()) } else { (-x, z.chars().rev().collect()) }
}
#[allow(warnings)]
fn f1_y<const y: bool>(x: i32, z: &str) -> (i32, String) {
    if y { (x, z.to_string()) } else { (-x, z.chars().rev().collect()) }
}
#[allow(warnings)]
fn f1_x_y<const x: i32, const y: bool>(z: &str) -> (i32, String) {
    if y { (x, z.to_string()) } else { (-x, z.chars().rev().collect()) }
}
#[inline(always)]
fn f1(x: i32, y: bool, z: &str) -> (i32, String) {
    match (x, y) {
        (1, false) => f1_x_y::<1, false>(z),
        (1, true) => f1_x_y::<1, true>(z),
        (0, false) => f1_x_y::<0, false>(z),
        (0, true) => f1_x_y::<0, true>(z),
        (x, false) => f1_y::<false>(x, z),
        (x, true) => f1_y::<true>(x, z),
        (1, y) => f1_x::<1>(y, z),
        (0, y) => f1_x::<0>(y, z),
        (x, y) => f1_orig(x, y, z),
        _ => {
            panic!("No matching branch");
        }
    }
}
```

The original function `f1` is renamed to `f1_orig`, and a powerset of two const arguments `x` and `y` are generated as different functions `f1_x`, `f1_y` and `f1_x_y`. Finally, the original function `f1` is replaced by a dispatcher function, which calls the generated functions according to the runtime values of `x` and `y`.

## Benifits

In most cases, the compiler optimization is trustworth enough to generate best codes for you. However, when your function is too complicated to inline, the compiler may not be able to get enough information to optimize the function.

The macro generates multiple versions of the function, and matches the consts argument explicitly in the dispatcher function. This enforces the compiler to generate the best codes for each const value. This is also why const-generics is introduced to rust, and the macro make it easier to use with a runtime-dependent value.

"No silver bullet" is a well-known principle in software engineering. The macro is not a silver bullet, and it's not always the best choice to use it. At least, it may heavily increase you binary size. You should always profile your code before and after using the macro, and make sure the performance is improved.

## License

Licensed under either of [Apache License, Version
2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
