//! Mouse camera for the 3D effects (Transform 3D, Luma Mesh): orbit around a target,
//! zoom by distance, pan in screen space. Stored in the effect (`camera` data), so presets
//! and the export use the view set with the mouse.

/// Effect data key holding the camera.
pub const DATA_KEY: &str = "camera";
/// Distance of the default camera from the plane.
pub const DEFAULT_DISTANCE: f32 = 886.0;
const MIN_DISTANCE: f32 = 10.0;
const MAX_DISTANCE: f32 = 4900.0;
/// Vertical field of view of the camera, degrees.
pub const FOV_DEG: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    /// rotation around the vertical axis, radians (0: looking along -z)
    pub azimuth: f32,
    /// angle from the up axis, radians (pi/2: level)
    pub polar: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

impl Default for Orbit {
    fn default() -> Self {
        Self { azimuth: 0.0, polar: std::f32::consts::FRAC_PI_2, distance: DEFAULT_DISTANCE, target: [0.0; 3] }
    }
}

impl Orbit {
    pub fn parse(s: &str) -> Option<Self> {
        let v: Vec<f32> = s.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
        let [azimuth, polar, distance, tx, ty, tz] = v[..] else { return None };
        if !v.iter().all(|x| x.is_finite()) {
            return None;
        }
        Some(Self { azimuth, polar, distance, target: [tx, ty, tz] })
    }

    pub fn to_data(&self) -> String {
        let [tx, ty, tz] = self.target;
        format!("{},{},{},{},{},{}", self.azimuth, self.polar, self.distance, tx, ty, tz)
    }

    /// Camera position.
    pub fn eye(&self) -> [f32; 3] {
        let s = self.polar.sin();
        let [tx, ty, tz] = self.target;
        [tx + self.distance * s * self.azimuth.sin(), ty + self.distance * self.polar.cos(), tz + self.distance * s * self.azimuth.cos()]
    }

    /// Drag with the primary button: `dx`, `dy` in pixels on a view `height` pixels tall.
    /// A drag across the full height turns the camera once around.
    pub fn rotate(&mut self, dx: f32, dy: f32, height: f32) {
        let k = std::f32::consts::TAU / height.max(1.0);
        self.azimuth -= dx * k;
        let eps = 1e-6;
        self.polar = (self.polar - dy * k).clamp(eps, std::f32::consts::PI - eps);
    }

    /// Mouse wheel: positive `steps` zoom in (each step 5 % closer).
    pub fn zoom(&mut self, steps: f32) {
        self.distance = (self.distance * 0.95f32.powf(steps)).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    /// Drag with the secondary button: the scene follows the pointer at the target's depth.
    pub fn pan(&mut self, dx: f32, dy: f32, height: f32) {
        let per_px = 2.0 * self.distance * (FOV_DEG.to_radians() / 2.0).tan() / height.max(1.0);
        // camera right and up vectors (up = world up projected on the view plane)
        let (sa, ca, sp, cp) = (self.azimuth.sin(), self.azimuth.cos(), self.polar.sin(), self.polar.cos());
        let right = [ca, 0.0, -sa];
        let up = [-cp * sa, sp, -cp * ca];
        for i in 0..3 {
            self.target[i] += (-dx * right[i] + dy * up[i]) * per_px;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-2)
    }

    #[test]
    fn default_camera_looks_down_the_z_axis() {
        assert!(close(Orbit::default().eye(), [0.0, 0.0, DEFAULT_DISTANCE]));
    }

    #[test]
    fn rotate_zoom_pan() {
        let mut o = Orbit::default();
        // a quarter of the view height to the left turns the camera a quarter to the right
        o.rotate(-250.0, 0.0, 1000.0);
        assert!(close(o.eye(), [DEFAULT_DISTANCE, 0.0, 0.0]), "{:?}", o.eye());
        // straight up stops just short of the pole
        o.rotate(0.0, 10_000.0, 1000.0);
        assert!(o.polar > 0.0 && o.polar < 1e-3);
        let mut o = Orbit::default();
        o.zoom(1.0);
        assert!((o.distance - DEFAULT_DISTANCE * 0.95).abs() < 1e-3);
        o.zoom(-1000.0);
        assert_eq!(o.distance, MAX_DISTANCE);
        // dragging right moves the scene right: the target moves left
        let mut o = Orbit::default();
        o.pan(100.0, 0.0, 1000.0);
        assert!(o.target[0] < 0.0 && o.target[1].abs() < 1e-3);
        o.pan(0.0, 100.0, 1000.0);
        assert!(o.target[1] > 0.0);
    }

    #[test]
    fn data_roundtrip() {
        let mut o = Orbit::default();
        o.rotate(12.3, -45.6, 700.0);
        o.pan(5.0, 7.0, 700.0);
        o.zoom(2.5);
        assert_eq!(Orbit::parse(&o.to_data()), Some(o));
        assert_eq!(Orbit::parse("1,2,3"), None);
        assert_eq!(Orbit::parse("1,2,NaN,0,0,0"), None);
    }
}
