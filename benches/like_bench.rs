use criterion::{black_box, criterion_group, criterion_main, Criterion};

use const_currying::const_currying;

#[const_currying]
fn like(
    s: &str,
    p: &str,
    #[maybe_const(consts = [b'\\'])] escape: u8,
    #[maybe_const(consts = [true, false])] case_sensitive: bool,
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
                    if ((!case_sensitive && pc == escape)
                        || (case_sensitive && pc.eq_ignore_ascii_case(&escape)))
                        && px + 1 < pbytes.len()
                    {
                        px += 1;
                        pc = pbytes[px];
                    }
                    if sx < sbytes.len()
                        && ((!case_sensitive && sbytes[sx] == pc)
                            || (case_sensitive && sbytes[sx].eq_ignore_ascii_case(&pc)))
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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("like base", |b| {
        b.iter(|| {
            like_orig(
                black_box("ab\\bbccddeefff"),
                black_box("ab\\%%%ccd*f"),
                black_box(b'\\'),
                black_box(true),
            )
        })
    });

    c.bench_function("like optimized", |b| {
        b.iter(|| {
            like(
                black_box("ab\\bbccddeefff"),
                black_box("ab\\%%%ccd*f"),
                black_box(b'\\'),
                black_box(true),
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
