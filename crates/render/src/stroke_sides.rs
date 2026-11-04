//! Per-side CSS border wedge geometry, AA seam pushes and transparent-first ordering.
use crate::{
    canvas::Canvas,
    color,
    stroke::{self, Edges},
    RenderError,
};
use opencanvas_layout::SceneStroke;
use tiny_skia::PathBuilder;

fn wedges(e: &Edges) -> [[[f32; 2]; 4]; 4] {
    let o = e.outer.corners();
    let mut inner = e.inner.corners();
    for i in 0..4 {
        let [h, v] = e.k[i];
        let [rx, ry] = e.inner_r[i];
        let t = if rx > 0.0 && ry > 0.0 && h + v > 0.0 {
            1.0 / (h / rx + v / ry)
        } else {
            0.0
        };
        inner[i][0] += (inner[i][0] - o[i][0]) * t;
        inner[i][1] += (inner[i][1] - o[i][1]) * t;
    }
    std::array::from_fn(|n| {
        let m = (n + 1) % 4;
        [o[n], o[m], inner[m], inner[n]]
    })
}
const NORMAL: [[f32; 2]; 4] = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]];
pub(crate) fn paint(
    c: &mut Canvas,
    e: &Edges,
    s: &SceneStroke,
    alpha: f32,
) -> Result<(), RenderError> {
    let mut order: Vec<usize> = (0..4).filter(|n| s.width[*n] > 0.0).collect();
    let a = [
        color::color(&s.color[0])?.alpha(),
        color::color(&s.color[1])?.alpha(),
        color::color(&s.color[2])?.alpha(),
        color::color(&s.color[3])?.alpha(),
    ];
    order.sort_by(|x, y| {
        a[*x]
            .total_cmp(&a[*y])
            .then((x % 2).cmp(&(y % 2)))
            .then(x.cmp(y))
    });
    let mut rank = [4; 4];
    for (i, n) in order.iter().enumerate() {
        rank[*n] = i;
    }
    let d = if order.iter().all(|n| a[*n] == 1.0) {
        1.0 / c.scale
    } else {
        0.0
    };
    let parts = wedges(e);
    for n in order {
        let shift = |p: [f32; 2], m: usize, corner: usize, out: f32| {
            let same = s.width[m] == 0.0 || s.color[m] == s.color[n];
            let square = e.outer_r[corner][0] == 0.0 || e.outer_r[corner][1] == 0.0;
            let s = if same || (square && rank[m] > rank[n]) {
                d
            } else {
                0.0
            };
            std::array::from_fn(|i| p[i] + s * (NORMAL[m][i] - NORMAL[n][i] + out * NORMAL[n][i]))
        };
        let [a, b, d, f] = parts[n];
        let prev = (n + 3) % 4;
        let next = (n + 1) % 4;
        let points: [[_; 2]; 4] = [
            shift(a, prev, n, 2.0),
            shift(b, next, next, 2.0),
            shift(d, next, next, 0.0),
            shift(f, prev, n, 0.0),
        ];
        let mut pb = PathBuilder::new();
        pb.move_to(points[0][0], points[0][1]);
        for p in &points[1..] {
            pb.line_to(p[0], p[1]);
        }
        pb.close();
        if let Some(p) = pb.finish() {
            let old = c.clip.clone();
            c.intersect_clip(&p)?;
            stroke::ring(c, e, &s.color[n], alpha, false)?;
            c.clip = old;
        }
    }
    Ok(())
}
