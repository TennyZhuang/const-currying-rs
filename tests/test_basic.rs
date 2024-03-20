use const_currying::const_currying;

#[const_currying]
fn f1(
    #[maybe_const(dispatch = x, consts = [0, 1])] x: i32,
    #[maybe_const(dispatch = y, consts = [true, false])] y: bool,
    z: &str,
) -> (i32, String) {
    if y {
        (x, z.to_string())
    } else {
        (-x, z.chars().rev().collect())
    }
}

fn main() {
    f1_orig(3, true, "3");
    f1(3, true, "3");
    f1_x::<3>(false, "3");
    f1_y::<true>(3, "3");
    f1_x_y::<3, true>("3");
}
