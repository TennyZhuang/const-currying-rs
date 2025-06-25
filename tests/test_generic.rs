use std::fmt::Display;

use const_currying::const_currying;

#[const_currying]
fn f1<T: Display>(#[maybe_const(dispatch = x, consts = [0, 1])] x: i32, y: T) -> String {
    if x > 0 {
        (y.to_string())
    } else {
        (y.to_string().chars().rev().collect())
    }
}

#[const_currying]
fn f2<const T: usize>(#[maybe_const(dispatch = x, consts = [0, 1])] x: i32) -> String {
    if x > 0 {
        (T.to_string())
    } else {
        (T.to_string())
    }
}

trait LfTrait<'a> {
    fn stringify(&self) -> String;
}
struct LfStruct<'a> {
    value: &'a str,
}
impl LfTrait<'_> for LfStruct<'_> {
    fn stringify(&self) -> String {
        self.value.to_string()
    }
}

#[const_currying]
fn f3<'a, T>(#[maybe_const(dispatch = x, consts = [0, 1])] x: i32, y: &'a T) -> String
where
    T: LfTrait<'a>,
{
    if x > 0 {
        y.stringify()
    } else {
        y.stringify().chars().rev().collect()
    }
}

#[const_currying]
fn f4<'a, T: ToString>(#[maybe_const(dispatch = x, consts = [0, 1])] x: i32, y: &'a T) -> String {
    if x > 0 {
        y.to_string()
    } else {
        y.to_string().chars().rev().collect()
    }
}

fn main() {
    f1_orig(0, "3");
    f1(0, "3");
    f1_x::<&str, 0>("3");

    f2_orig::<0usize>(0);
    f2::<0usize>(0);
    f2_x::<0usize, 0>();
    let lf = LfStruct { value: "3" };

    f3_orig(0, &lf);
    f3(0, &lf);
    f3_x::<LfStruct, 0>(&lf);

    f4_orig(0, &"3");
    f4(0, &"3");
    f4_x::<&str, 0>(&"3");
}
