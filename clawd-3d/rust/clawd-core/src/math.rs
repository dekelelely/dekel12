//! Vectors and 4×4 matrices the way three.js does them: column-major, Euler angles in its "XYZ" and "YXZ" orders, an
//! OpenGL-style perspective camera. f64 throughout, like the JavaScript it follows.
use core::ops::{Add, Mul, Neg, Sub};
use libm::{cos, sin, sqrt, tan};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn length(self) -> f64 {
        sqrt(self.dot(self))
    }
    pub fn normalize(self) -> Vec3 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            self
        }
    }
    /// Turned about an axis through the origin (three.js applyAxisAngle).
    pub fn rotate_about(self, axis: Vec3, angle: f64) -> Vec3 {
        Mat4::rotation_axis(axis, angle).transform_dir(self)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        v3(-self.x, -self.y, -self.z)
    }
}

/// Euler angle order, as three.js names them: "XYZ" (the default) or "YXZ".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Xyz,
    Yxz,
}

/// A 4×4 matrix, column-major like three.js's `elements`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [f64; 16]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]);

    pub fn translation(t: Vec3) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.0[12] = t.x;
        m.0[13] = t.y;
        m.0[14] = t.z;
        m
    }

    pub fn scaling(s: Vec3) -> Mat4 {
        Mat4([s.x, 0., 0., 0., 0., s.y, 0., 0., 0., 0., s.z, 0., 0., 0., 0., 1.])
    }

    /// three.js Matrix4.makeRotationFromEuler.
    pub fn rotation(x: f64, y: f64, z: f64, order: Order) -> Mat4 {
        let (a, b, c, d, e, f) = (cos(x), sin(x), cos(y), sin(y), cos(z), sin(z));
        let mut t = [0.0; 16];
        match order {
            Order::Xyz => {
                let (ae, af, be, bf) = (a * e, a * f, b * e, b * f);
                t[0] = c * e;
                t[4] = -c * f;
                t[8] = d;
                t[1] = af + be * d;
                t[5] = ae - bf * d;
                t[9] = -b * c;
                t[2] = bf - ae * d;
                t[6] = be + af * d;
                t[10] = a * c;
            }
            Order::Yxz => {
                let (ce, cf, de, df) = (c * e, c * f, d * e, d * f);
                t[0] = ce + df * b;
                t[4] = de * b - cf;
                t[8] = a * d;
                t[1] = a * f;
                t[5] = a * e;
                t[9] = -b;
                t[2] = cf * b - de;
                t[6] = df + ce * b;
                t[10] = a * c;
            }
        }
        t[15] = 1.0;
        Mat4(t)
    }

    /// A turn about an axis through the origin (three.js makeRotationAxis).
    pub fn rotation_axis(axis: Vec3, angle: f64) -> Mat4 {
        let a = axis.normalize();
        let (c, s) = (cos(angle), sin(angle));
        let t = 1.0 - c;
        let (x, y, z) = (a.x, a.y, a.z);
        let (tx, ty) = (t * x, t * y);
        Mat4([
            tx * x + c,
            tx * y + s * z,
            tx * z - s * y,
            0.,
            tx * y - s * z,
            ty * y + c,
            ty * z + s * x,
            0.,
            tx * z + s * y,
            ty * z - s * x,
            t * z * z + c,
            0.,
            0.,
            0.,
            0.,
            1.,
        ])
    }

    /// Position, Euler rotation and scale, as an Object3D's matrix: T · R · S.
    pub fn compose(pos: Vec3, rot: Vec3, order: Order, scale: Vec3) -> Mat4 {
        Mat4::translation(pos) * Mat4::rotation(rot.x, rot.y, rot.z, order) * Mat4::scaling(scale)
    }

    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let m = &self.0;
        v3(
            m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
            m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
            m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
        )
    }

    pub fn transform_dir(&self, d: Vec3) -> Vec3 {
        let m = &self.0;
        v3(
            m[0] * d.x + m[4] * d.y + m[8] * d.z,
            m[1] * d.x + m[5] * d.y + m[9] * d.z,
            m[2] * d.x + m[6] * d.y + m[10] * d.z,
        )
    }

    /// A point through a projection: x, y, z divided by w (three.js applyMatrix4), and w.
    pub fn project(&self, p: Vec3) -> (Vec3, f64) {
        let m = &self.0;
        let w = m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15];
        let q = self.transform_point(p);
        (q * (1.0 / w), w)
    }

    pub fn origin(&self) -> Vec3 {
        v3(self.0[12], self.0[13], self.0[14])
    }

    pub fn invert(&self) -> Option<Mat4> {
        let m = &self.0;
        let mut inv = [0.0; 16];
        inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
            - m[9] * m[3] * m[14]
            - m[13] * m[2] * m[11]
            + m[13] * m[3] * m[10];
        inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
            + m[8] * m[3] * m[14]
            + m[12] * m[2] * m[11]
            - m[12] * m[3] * m[10];
        inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
            - m[8] * m[3] * m[13]
            - m[12] * m[1] * m[11]
            + m[12] * m[3] * m[9];
        inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
            + m[8] * m[2] * m[13]
            + m[12] * m[1] * m[10]
            - m[12] * m[2] * m[9];
        inv[2] =
            m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] + m[13] * m[2] * m[7]
                - m[13] * m[3] * m[6];
        inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
            - m[4] * m[3] * m[14]
            - m[12] * m[2] * m[7]
            + m[12] * m[3] * m[6];
        inv[10] =
            m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] + m[12] * m[1] * m[7]
                - m[12] * m[3] * m[5];
        inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
            - m[4] * m[2] * m[13]
            - m[12] * m[1] * m[6]
            + m[12] * m[2] * m[5];
        inv[3] =
            -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] - m[9] * m[2] * m[7]
                + m[9] * m[3] * m[6];
        inv[7] =
            m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] + m[8] * m[2] * m[7]
                - m[8] * m[3] * m[6];
        inv[11] =
            -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] - m[8] * m[1] * m[7]
                + m[8] * m[3] * m[5];
        inv[15] =
            m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6]
                - m[8] * m[2] * m[5];
        let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
        if det == 0.0 {
            return None;
        }
        let r = 1.0 / det;
        for v in inv.iter_mut() {
            *v *= r;
        }
        Some(Mat4(inv))
    }

    /// three.js PerspectiveCamera's projection: vertical field of view in degrees.
    pub fn perspective(fov_deg: f64, aspect: f64, near: f64, far: f64) -> Mat4 {
        let top = near * tan(fov_deg.to_radians() / 2.0);
        let (bottom, right) = (-top, aspect * top);
        let left = -right;
        let x = 2.0 * near / (right - left);
        let y = 2.0 * near / (top - bottom);
        let a = (right + left) / (right - left);
        let b = (top + bottom) / (top - bottom);
        let c = -(far + near) / (far - near);
        let d = -2.0 * far * near / (far - near);
        Mat4([x, 0., 0., 0., 0., y, 0., 0., a, b, c, -1., 0., 0., d, 0.])
    }

    /// three.js OrthographicCamera's projection.
    pub fn orthographic(left: f64, right: f64, top: f64, bottom: f64, near: f64, far: f64) -> Mat4 {
        let (w, h, p) = (1.0 / (right - left), 1.0 / (top - bottom), 1.0 / (far - near));
        let (x, y, z) = ((right + left) * w, (top + bottom) * h, (far + near) * p);
        Mat4([2. * w, 0., 0., 0., 0., 2. * h, 0., 0., 0., 0., -2. * p, 0., -x, -y, -z, 1.])
    }

    /// The world matrix of a camera at `eye` looking at `target` (three.js lookAt for cameras, up +y).
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        let mut z = eye - target;
        if z.length() == 0.0 {
            z.z = 1.0;
        }
        z = z.normalize();
        let mut x = up.cross(z);
        if x.length() == 0.0 {
            // up and z are parallel: nudge z, as three.js does
            if up.z.abs() == 1.0 {
                z.x += 0.0001;
            } else {
                z.z += 0.0001;
            }
            z = z.normalize();
            x = up.cross(z);
        }
        x = x.normalize();
        let y = z.cross(x);
        Mat4([x.x, x.y, x.z, 0., y.x, y.y, y.z, 0., z.x, z.y, z.z, 0., eye.x, eye.y, eye.z, 1.])
    }
}

impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let (a, b) = (&self.0, &o.0);
        let mut r = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                r[col * 4 + row] = a[row] * b[col * 4]
                    + a[4 + row] * b[col * 4 + 1]
                    + a[8 + row] * b[col * 4 + 2]
                    + a[12 + row] * b[col * 4 + 3];
            }
        }
        Mat4(r)
    }
}

pub fn clamp(v: f64, a: f64, b: f64) -> f64 {
    if v < a {
        a
    } else if v > b {
        b
    } else {
        v
    }
}

/// JavaScript's Math.round: halves round up (toward +∞).
pub fn js_round(v: f64) -> f64 {
    libm::floor(v + 0.5)
}

/// An sRGB colour ("#d77757") in linear light, as three.js's convertSRGBToLinear.
pub const fn hex(rgb: u32) -> [f64; 3] {
    [((rgb >> 16) & 255) as f64 / 255.0, ((rgb >> 8) & 255) as f64 / 255.0, (rgb & 255) as f64 / 255.0]
}

pub fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 {
        c * 0.0773993808
    } else {
        libm::pow(c * 0.9478672986 + 0.0521327014, 2.4)
    }
}

pub fn lin(rgb: u32) -> [f32; 3] {
    let c = hex(rgb);
    [srgb_to_linear(c[0]) as f32, srgb_to_linear(c[1]) as f32, srgb_to_linear(c[2]) as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-9
    }

    #[test]
    fn rotations_match_three_js() {
        // A quarter turn about y takes +x to -z; YXZ applies y last.
        let m = Mat4::rotation(0.0, core::f64::consts::FRAC_PI_2, 0.0, Order::Yxz);
        assert!(close(m.transform_dir(v3(1., 0., 0.)), v3(0., 0., -1.)));
        let m = Mat4::rotation(core::f64::consts::FRAC_PI_2, 0.0, 0.0, Order::Xyz);
        assert!(close(m.transform_dir(v3(0., 1., 0.)), v3(0., 0., 1.)));
        let axis = Mat4::rotation_axis(v3(0., 1., 0.), 0.7);
        assert!(close(
            axis.transform_dir(v3(1., 2., 3.)),
            Mat4::rotation(0., 0.7, 0., Order::Xyz).transform_dir(v3(1., 2., 3.))
        ));
        let m = Mat4::compose(v3(1., 2., 3.), v3(0.3, -0.2, 0.9), Order::Yxz, v3(2., 1., 0.5));
        let p = v3(0.4, -1.0, 2.0);
        assert!(close(m.invert().unwrap().transform_point(m.transform_point(p)), p));
    }

    #[test]
    fn projection_puts_the_target_in_the_middle() {
        let view = Mat4::look_at(v3(10., 5., 20.), v3(-3., 7., 0.), v3(0., 1., 0.)).invert().unwrap();
        let proj = Mat4::perspective(30.0, 1.5, 0.5, 400.0);
        let (p, _) = (proj * view).project(v3(-3., 7., 0.));
        assert!(p.x.abs() < 1e-9 && p.y.abs() < 1e-9 && p.z > -1.0 && p.z < 1.0);
    }
}
