
#[path = "printhor/math/mod.rs"]
mod math;
pub type Real = math::Real;
pub use math::RealInclusiveRange;

#[path = "printhor/tgeo.rs"]
mod tgeo;
pub use tgeo::*;

#[path = "printhor/ctrl.rs"]
pub mod ctrl;

#[path = "printhor/planner/mod.rs"]
pub mod planner;

#[path = "printhor/hwa/controllers/motion/motion_segment.rs"]
mod motion_segment;

use motion_segment::{Segment, SegmentData};
use planner::{Constraints, PlanProfile, SCurveMotionProfile};
#[allow(unused)]
use crate::math::{ONE, ZERO};

mod hwa {
    pub use printhor_hwi_native::{trace, debug, info, warn, error};
    pub use printhor_hwi_native::init_logger;
}

#[path = "printhor/geometry.rs"]
mod geometry;

extern crate alloc;

fn main() {

    hwa::init_logger();

    let dts = Real::from_f32(300.0); // default_travel_speed
    let flow_rate = Real::one();
    let speed_rate = Real::one();
    let max_speed = TVector::from_coords(
        Some(Real::from_f32(800.0)),
        Some(Real::from_f32(8.0)),
        Some(Real::from_f32(8.0)),
        Some(Real::from_f32(2.0)),
    );
    let max_accel = TVector::from_coords(
        Some(Real::from_f32(64.0)),
        Some(Real::from_f32(64.0)),
        Some(Real::from_f32(64.0)),
        Some(Real::from_f32(16.0)),
    );

    let max_jerk = TVector::from_coords(
        Some(Real::from_f32(128.0)),
        Some(Real::from_f32(128.0)),
        Some(Real::from_f32(128.0)),
        Some(Real::from_f32(32.0)),
    );

    let requested_motion_speed = Some(Real::from_f32(700.0f32));

    let p0: TVector<Real> = TVector::zero();
    let p1: TVector<Real> = TVector::from_coords(
        Some(Real::from_f32(20.0)),
        None,
        None,
        Some(Real::from_f32(1.0)));

    // Compute distance and decompose as unit vector and module.
    // When dist is zero, value is map to None (NaN).
    // In case o E dimension, flow rate factor is applied
    let (vdir, module_target_distance) = (p1 - p0)
        .map_coord(CoordSel::all(), |coord_value, coord_idx| {
            match coord_idx {
                CoordSel::X | CoordSel::Y | CoordSel::Z => {
                    match coord_value.is_zero() {
                        true => None,
                        false => Some(coord_value),
                    }
                },
                _ => None,
            }
        }).decompose_normal();

    // Compute the speed module applying speed_rate factor
    let speed_module = requested_motion_speed.unwrap_or(dts) * speed_rate;
    // Compute per-axis target speed
    let speed_vector: TVector<Real> = vdir.with_coord(CoordSel::E,
        p1.e.and_then(|v| {
            match v.is_zero() {
                true => None,
                false => Some(v * flow_rate),
            }

        })
    ).abs() * speed_module;
    // Clamp per-axis target speed to the physical restrictions
    let clamped_speed = speed_vector.clamp(max_speed);
    // Finally, per-axis relative speed

    let v = vdir * module_target_distance;

    hwa::info!("v: {}", v);
    hwa::info!("speed_vector: {}", speed_vector);
    hwa::info!("speed_vector / v: {}", speed_vector / v);
    hwa::info!("max_speed: {}", max_speed);
    hwa::info!("max_speed_v: {}", speed_vector * vdir);
    hwa::info!("speed_vector / max_speed: {}", speed_vector / max_speed);
    hwa::info!("clamped_speed: {}", clamped_speed);

    let module_target_speed = clamped_speed.min().unwrap();
    let module_target_accel = max_accel.norm2().unwrap();
    let module_target_jerk = max_jerk.norm2().unwrap();

    if !module_target_distance.is_zero() {
        match !module_target_speed.is_zero() && !module_target_accel.is_zero() && !module_target_jerk.is_zero() {
            true => {
                let t2 = embassy_time::Instant::now();
                let _constraints = Constraints {
                    v_max: module_target_speed,
                    a_max: module_target_accel,
                    j_max: module_target_jerk,
                };
                let segment = Segment::new(
                    SegmentData {
                        speed_enter_sps: 0,
                        speed_exit_sps: 0,
                        total_steps: module_target_distance.to_i32().unwrap_or(0) as u32,
                        vdir,
                        dest_pos: Default::default(),
                    },
                    SCurveMotionProfile::compute(
                        module_target_distance, ZERO, ZERO,
                        &_constraints).unwrap()
                );
                let t3 = embassy_time::Instant::now();

                hwa::info!("----");
                hwa::info!("P0 ({})", p0);
                hwa::info!("P1 ({})", p1);
                hwa::info!("--");
                hwa::info!("dist: {}", module_target_distance.rdp(4));
                hwa::info!("speed: {}", module_target_speed.rdp(4));
                hwa::info!("accel: {}", module_target_accel.rdp(4));
                hwa::info!("jerk: {}", module_target_jerk.rdp(4));
                hwa::info!("--");
                hwa::info!("plan computed in : {} us", (t3-t2).as_micros());

                PlanProfile::new(segment.motion_profile, Real::from_f32(0.01))
                    .plot(true, true, true, true);
            },
            false => {
                hwa::error!("p0: {}", p0.rdp(4));
                hwa::error!("p1: {}", p1.rdp(4));
                hwa::error!("dist: {}", module_target_distance.rdp(4));
                hwa::error!("vdir: {}", vdir.rdp(4));
                hwa::error!("speed_vector: {}", speed_vector.rdp(4));
                hwa::error!("clamped_speed: {}", clamped_speed.rdp(4));
                hwa::error!("clamped_speed: {}", clamped_speed.rdp(4));
            }
        };
    }
}
