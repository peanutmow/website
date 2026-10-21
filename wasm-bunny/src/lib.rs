use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

const MESH_BYTES: &[u8] = include_bytes!("../assets/bunny.bin");

struct Mesh {
    verts: Vec<i16>,
    normals: Vec<i16>,
    faces: Vec<u16>,
    face_count: usize,
}

fn mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(|| parse_mesh(MESH_BYTES))
}

fn parse_mesh(bytes: &[u8]) -> Mesh {
    if bytes.len() < 12 || &bytes[0..4] != b"BNY1" {
        return Mesh {
            verts: Vec::new(),
            normals: Vec::new(),
            faces: Vec::new(),
            face_count: 0,
        };
    }
    let verts = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let faces = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let mut offset = 12;

    let mut read_i16 = |count: usize| -> Vec<i16> {
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(i16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
            offset += 2;
        }
        out
    };
    let positions = read_i16(verts * 3);
    let normals = read_i16(verts * 3);

    let mut indices = Vec::with_capacity(faces * 3);
    for _ in 0..faces * 3 {
        indices.push(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
        offset += 2;
    }

    Mesh {
        verts: positions,
        normals,
        faces: indices,
        face_count: faces,
    }
}

fn unit(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let length = (x * x + y * y + z * z).sqrt();
    if length == 0.0 {
        (x, y, z)
    } else {
        (x / length, y / length, z / length)
    }
}

pub fn render(
    cols: i64,
    rows: i64,
    angle_x: f64,
    angle_y: f64,
    ramp: &str,
    invert: bool,
) -> Vec<String> {
    let mesh = mesh();
    let c = cols.max(6) as usize;
    let r = rows.max(6) as usize;
    let chars: Vec<char> = if ramp.is_empty() {
        " .-:~,^;*=|>+\\/?&$%#@".chars().collect()
    } else {
        ramp.chars().collect()
    };
    let last = chars.len().saturating_sub(1);

    let mut zbuf = vec![f64::NEG_INFINITY; c * r];
    let mut nbuf = vec![0.0_f64; c * r * 3];

    let (kx, ky, kz) = unit(0.6, 0.8, 0.5);
    let (ax, ay, az) = unit(-0.5, 0.3, 0.7);
    let (bx, by, bz) = unit(0.0, -0.5, -0.8);

    let sa = angle_x.sin();
    let ca = angle_x.cos();
    let sb = angle_y.sin();
    let cb = angle_y.cos();
    let m0 = cb;
    let m1 = 0.0;
    let m2 = sb;
    let m3 = sa * sb;
    let m4 = ca;
    let m5 = -sa * cb;
    let m6 = -ca * sb;
    let m7 = sa;
    let m8 = ca * cb;

    let width = (c - 1) as f64;
    let height = (r - 1) as f64;
    let sx = width / (c - 2) as f64;
    let sy = height / (r - 2) as f64;

    for face in 0..mesh.face_count {
        let ai = mesh.faces[face * 3] as usize;
        let bi = mesh.faces[face * 3 + 1] as usize;
        let ci = mesh.faces[face * 3 + 2] as usize;

        let va0 = f64::from(mesh.verts[ai * 3]);
        let va1 = f64::from(mesh.verts[ai * 3 + 1]);
        let va2 = f64::from(mesh.verts[ai * 3 + 2]);
        let vb0 = f64::from(mesh.verts[bi * 3]);
        let vb1 = f64::from(mesh.verts[bi * 3 + 1]);
        let vb2 = f64::from(mesh.verts[bi * 3 + 2]);
        let vc0 = f64::from(mesh.verts[ci * 3]);
        let vc1 = f64::from(mesh.verts[ci * 3 + 1]);
        let vc2 = f64::from(mesh.verts[ci * 3 + 2]);

        let x_ax = (m0 * va0 + m1 * va1 + m2 * va2) * 0.001;
        let y_ax = (m3 * va0 + m4 * va1 + m5 * va2) * 0.001;
        let z_ax = (m6 * va0 + m7 * va1 + m8 * va2) * 0.001;
        let x_bx = (m0 * vb0 + m1 * vb1 + m2 * vb2) * 0.001;
        let y_bx = (m3 * vb0 + m4 * vb1 + m5 * vb2) * 0.001;
        let z_bx = (m6 * vb0 + m7 * vb1 + m8 * vb2) * 0.001;
        let x_cx = (m0 * vc0 + m1 * vc1 + m2 * vc2) * 0.001;
        let y_cx = (m3 * vc0 + m4 * vc1 + m5 * vc2) * 0.001;
        let z_cx = (m6 * vc0 + m7 * vc1 + m8 * vc2) * 0.001;

        let pax = (x_ax / sx + 1.0) * 0.5 * width;
        let pay = (1.0 - y_ax / sy) * 0.5 * height;
        let pbx = (x_bx / sx + 1.0) * 0.5 * width;
        let pby = (1.0 - y_bx / sy) * 0.5 * height;
        let pcx = (x_cx / sx + 1.0) * 0.5 * width;
        let pcy = (1.0 - y_cx / sy) * 0.5 * height;

        let area = (pbx - pax) * (pcy - pay) - (pby - pay) * (pcx - pax);
        if area >= 0.0 {
            continue;
        }

        let x0 = (pax.min(pbx).min(pcx).floor()).max(0.0) as usize;
        let x1 = (pax.max(pbx).max(pcx).ceil()).min(width) as usize;
        let y0 = (pay.min(pby).min(pcy).floor()).max(0.0) as usize;
        let y1 = (pay.max(pby).max(pcy).ceil()).min(height) as usize;
        if x0 > x1 || y0 > y1 {
            continue;
        }

        let na0 = f64::from(mesh.normals[ai * 3]);
        let na1 = f64::from(mesh.normals[ai * 3 + 1]);
        let na2 = f64::from(mesh.normals[ai * 3 + 2]);
        let nb0 = f64::from(mesh.normals[bi * 3]);
        let nb1 = f64::from(mesh.normals[bi * 3 + 1]);
        let nb2 = f64::from(mesh.normals[bi * 3 + 2]);
        let nc0 = f64::from(mesh.normals[ci * 3]);
        let nc1 = f64::from(mesh.normals[ci * 3 + 1]);
        let nc2 = f64::from(mesh.normals[ci * 3 + 2]);

        let nax = m0 * na0 + m1 * na1 + m2 * na2;
        let nay = m3 * na0 + m4 * na1 + m5 * na2;
        let naz = m6 * na0 + m7 * na1 + m8 * na2;
        let nbx = m0 * nb0 + m1 * nb1 + m2 * nb2;
        let nby = m3 * nb0 + m4 * nb1 + m5 * nb2;
        let nbz = m6 * nb0 + m7 * nb1 + m8 * nb2;
        let ncx = m0 * nc0 + m1 * nc1 + m2 * nc2;
        let ncy = m3 * nc0 + m4 * nc1 + m5 * nc2;
        let ncz = m6 * nc0 + m7 * nc1 + m8 * nc2;

        let inv = 1.0 / area;
        for py in y0..=y1 {
            for px in x0..=x1 {
                let fx = px as f64;
                let fy = py as f64;
                let w0 = (pcx - pbx) * (fy - pby) - (pcy - pby) * (fx - pbx);
                let w1 = (pax - pcx) * (fy - pcy) - (pay - pcy) * (fx - pcx);
                let w2 = (pbx - pax) * (fy - pay) - (pby - pay) * (fx - pax);
                if w0 > 0.0 || w1 > 0.0 || w2 > 0.0 {
                    continue;
                }

                let ba = w0 * inv;
                let bb = w1 * inv;
                let bc = w2 * inv;
                let z = z_ax * ba + z_bx * bb + z_cx * bc;
                let p = py * c + px;
                if z <= zbuf[p] {
                    continue;
                }
                zbuf[p] = z;
                nbuf[p * 3] = nax * ba + nbx * bb + ncx * bc;
                nbuf[p * 3 + 1] = nay * ba + nby * bb + ncy * bc;
                nbuf[p * 3 + 2] = naz * ba + nbz * bb + ncz * bc;
            }
        }
    }

    let mut lines = Vec::with_capacity(r);
    for row in 0..r {
        let mut line = String::with_capacity(c);
        for col in 0..c {
            let p = row * c + col;
            if zbuf[p] == f64::NEG_INFINITY {
                line.push(' ');
                continue;
            }

            let mut nx = nbuf[p * 3];
            let mut ny = nbuf[p * 3 + 1];
            let mut nz = nbuf[p * 3 + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            if length != 0.0 {
                nx /= length;
                ny /= length;
                nz /= length;
            }

            let mut lum = 0.07
                + (nx * kx + ny * ky + nz * kz).max(0.0) * 0.78
                + (nx * ax + ny * ay + nz * az).max(0.0) * 0.28
                + (nx * bx + ny * by + nz * bz).max(0.0) * 0.12;

            let spec = nx * kx + ny * ky + nz * kz;
            if spec > 0.0 {
                let rz = 2.0 * spec * nz - kz;
                if rz > 0.0 {
                    lum += rz.powf(18.0) * 0.55;
                }
            }

            let value = if invert { 1.0 - lum } else { lum };
            let shade = (value.clamp(0.0, 1.0) * last as f64).round() as usize;
            line.push(chars[shade.clamp(1, last)]);
        }
        lines.push(line.trim_end().to_string());
    }
    lines
}

#[wasm_bindgen]
pub fn bunny_render(
    cols: u32,
    rows: u32,
    angle_x: f64,
    angle_y: f64,
    ramp: &str,
    invert: bool,
) -> String {
    render(
        i64::from(cols),
        i64::from(rows),
        angle_x,
        angle_y,
        ramp,
        invert,
    )
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_is_parsed() {
        let mesh = mesh();
        assert_eq!(mesh.verts.len(), 2503 * 3);
        assert_eq!(mesh.normals.len(), 2503 * 3);
        assert_eq!(mesh.faces.len(), 4968 * 3);
    }

    #[test]
    fn render_returns_the_requested_grid() {
        let lines = render(40, 24, 0.08, -0.15, " .,-:~;=+*#%@", false);
        assert_eq!(lines.len(), 24);
        assert!(lines.iter().any(|line| !line.is_empty()));
        assert!(lines.iter().all(|line| line.chars().count() <= 40));
    }
}
