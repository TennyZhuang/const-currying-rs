use const_currying::const_currying;

#[const_currying]
fn f1(
    #[maybe_const(dispatch = x, consts = [0, 1])] x: i32,
    #[maybe_const(dispatch = y, consts = [true, false])] y: bool,
    z: i64,
) -> i32 {
    if y {
        x
    } else {
        -x
    }
}

fn main() {
    f1_orig(3, true, 3);
    f1(3, true, 3);
    f1_x::<3>(false, 3);
    f1_y::<true>(3, 3);
    f1_x_y::<3, true>(3);
}
