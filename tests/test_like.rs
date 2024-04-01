use const_currying::const_currying;

#[const_currying]
fn like_impl<const CASE_INSENSITIVE: bool>(
    s: &str,
    p: &str,
    #[maybe_const(consts = [b'\\'])] escape: u8,
) -> bool {
    let (mut px, mut sx) = (0, 0);
    let (mut next_px, mut next_sx) = (0, 0);
    let (pbytes, sbytes) = (p.as_bytes(), s.as_bytes());
    while px < pbytes.len() || sx < sbytes.len() {
        if px < pbytes.len() {
            let c = pbytes[px];
            match c {
                b'_' => {
                    if sx < sbytes.len() {
                        px += 1;
                        sx += 1;
                        continue;
                    }
                }
                b'%' => {
                    next_px = px;
                    next_sx = sx + 1;
                    px += 1;
                    continue;
                }
                mut pc => {
                    if ((!CASE_INSENSITIVE && pc == escape)
                        || (CASE_INSENSITIVE && pc.eq_ignore_ascii_case(&escape)))
                        && px + 1 < pbytes.len()
                    {
                        px += 1;
                        pc = pbytes[px];
                    }
                    if sx < sbytes.len()
                        && ((!CASE_INSENSITIVE && sbytes[sx] == pc)
                            || (CASE_INSENSITIVE && sbytes[sx].eq_ignore_ascii_case(&pc)))
                    {
                        px += 1;
                        sx += 1;
                        continue;
                    }
                }
            }
        }
        if 0 < next_sx && next_sx <= sbytes.len() {
            px = next_px;
            sx = next_sx;
            continue;
        }
        return false;
    }
    true
}

pub fn like_default(s: &str, p: &str) -> bool {
    like_impl_escape::<false, b'\\'>(s, p)
}

pub fn i_like_default(s: &str, p: &str) -> bool {
    like_impl_escape::<true, b'\\'>(s, p)
}
