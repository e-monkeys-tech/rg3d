//! A module which contains data model for serialization.
//!
//! We have to use custom serialization format, because we can't rely on backward
//! compatibility of Rapier.

use crate::{
    core::{
        algebra::{
            DMatrix, Dynamic, Isometry3, Point3, Translation, Translation3, Unit, UnitQuaternion,
            VecStorage, Vector3,
        },
        pool::ErasedHandle,
        visitor::prelude::*,
        BiDirHashMap,
    },
    engine::{ColliderHandle, JointHandle, RigidBodyHandle},
};
use rapier3d::{
    dynamics::{
        BallJoint, FixedJoint, IntegrationParameters, Joint, JointParams, PrismaticJoint,
        RevoluteJoint, RigidBody, RigidBodyBuilder, RigidBodyType,
    },
    geometry::{Collider, ColliderBuilder, InteractionGroups, Segment, Shape, SharedShape},
    math::AngVector,
};
use std::{collections::HashMap, hash::Hash};

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
#[doc(hidden)]
pub enum RigidBodyTypeDesc {
    Dynamic = 0,
    Static = 1,
    KinematicPositionBased = 2,
    KinematicVelocityBased = 3,
}

impl Default for RigidBodyTypeDesc {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl RigidBodyTypeDesc {
    fn id(self) -> u32 {
        self as u32
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Dynamic),
            1 => Ok(Self::Static),
            2 => Ok(Self::KinematicPositionBased),
            3 => Ok(Self::KinematicVelocityBased),
            _ => Err(format!("Invalid body status id {}!", id)),
        }
    }
}

impl Visit for RigidBodyTypeDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut id = self.id();
        id.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        Ok(())
    }
}

impl From<RigidBodyType> for RigidBodyTypeDesc {
    fn from(s: RigidBodyType) -> Self {
        match s {
            RigidBodyType::Dynamic => Self::Dynamic,
            RigidBodyType::Static => Self::Static,
            RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

impl Into<RigidBodyType> for RigidBodyTypeDesc {
    fn into(self) -> RigidBodyType {
        match self {
            RigidBodyTypeDesc::Dynamic => RigidBodyType::Dynamic,
            RigidBodyTypeDesc::Static => RigidBodyType::Static,
            RigidBodyTypeDesc::KinematicPositionBased => RigidBodyType::KinematicPositionBased,
            RigidBodyTypeDesc::KinematicVelocityBased => RigidBodyType::KinematicVelocityBased,
        }
    }
}

#[derive(Clone, Debug, Visit)]
#[doc(hidden)]
pub struct RigidBodyDesc<C> {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub lin_vel: Vector3<f32>,
    pub ang_vel: Vector3<f32>,
    pub sleeping: bool,
    pub status: RigidBodyTypeDesc,
    pub colliders: Vec<C>,
    pub mass: f32,
    pub x_rotation_locked: bool,
    pub y_rotation_locked: bool,
    pub z_rotation_locked: bool,
    pub translation_locked: bool,
}

impl<C> Default for RigidBodyDesc<C> {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
            lin_vel: Default::default(),
            ang_vel: Default::default(),
            sleeping: false,
            status: Default::default(),
            colliders: vec![],
            mass: 1.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
        }
    }
}

impl<C: Hash + Clone + Eq> RigidBodyDesc<C> {
    #[doc(hidden)]
    pub fn from_body(
        body: &RigidBody,
        handle_map: &BiDirHashMap<C, rapier3d::geometry::ColliderHandle>,
    ) -> Self {
        let rotation_locked = body.is_rotation_locked();
        Self {
            position: body.position().translation.vector,
            rotation: body.position().rotation,
            lin_vel: *body.linvel(),
            ang_vel: *body.angvel(),
            status: body.body_type().into(),
            sleeping: body.is_sleeping(),
            colliders: body
                .colliders()
                .iter()
                .map(|c| handle_map.key_of(c).cloned().unwrap())
                .collect(),
            mass: body.mass(),
            x_rotation_locked: rotation_locked[0],
            y_rotation_locked: rotation_locked[1],
            z_rotation_locked: rotation_locked[2],
            translation_locked: body.is_translation_locked(),
        }
    }

    pub(super) fn convert_to_body(self) -> RigidBody {
        let mut builder = RigidBodyBuilder::new(self.status.into())
            .position(Isometry3 {
                translation: Translation {
                    vector: self.position,
                },
                rotation: self.rotation,
            })
            .additional_mass(self.mass)
            .linvel(self.lin_vel)
            .angvel(AngVector::new(
                self.ang_vel.x,
                self.ang_vel.y,
                self.ang_vel.z,
            ))
            .restrict_rotations(
                self.x_rotation_locked,
                self.y_rotation_locked,
                self.z_rotation_locked,
            );

        if self.translation_locked {
            builder = builder.lock_translations();
        }

        let mut body = builder.build();
        if self.sleeping {
            body.sleep();
        }
        body
    }
}

impl<C> RigidBodyDesc<C> {
    #[doc(hidden)]
    pub fn local_transform(&self) -> Isometry3<f32> {
        Isometry3 {
            rotation: self.rotation,
            translation: Translation {
                vector: self.position,
            },
        }
    }
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct BallDesc {
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct CylinderDesc {
    pub half_height: f32,
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct RoundCylinderDesc {
    pub half_height: f32,
    pub radius: f32,
    pub border_radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct ConeDesc {
    pub half_height: f32,
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct CuboidDesc {
    pub half_extents: Vector3<f32>,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct CapsuleDesc {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
    pub radius: f32,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct SegmentDesc {
    pub begin: Vector3<f32>,
    pub end: Vector3<f32>,
}

#[derive(Default, Copy, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct TriangleDesc {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
}

// TODO: for now data of trimesh and heightfield is not serializable.
//  In most cases it is ok, because PhysicsBinder allows to automatically
//  obtain data from associated mesh.
#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct TrimeshDesc;

impl Visit for TrimeshDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
#[doc(hidden)]
pub struct HeightfieldDesc;

impl Visit for HeightfieldDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        visitor.leave_region()
    }
}

#[derive(Copy, Clone, Debug)]
#[doc(hidden)]
pub enum ColliderShapeDesc {
    Ball(BallDesc),
    Cylinder(CylinderDesc),
    RoundCylinder(RoundCylinderDesc),
    Cone(ConeDesc),
    Cuboid(CuboidDesc),
    Capsule(CapsuleDesc),
    Segment(SegmentDesc),
    Triangle(TriangleDesc),
    Trimesh(TrimeshDesc),
    Heightfield(HeightfieldDesc),
}

impl Default for ColliderShapeDesc {
    fn default() -> Self {
        Self::Ball(Default::default())
    }
}

impl ColliderShapeDesc {
    #[doc(hidden)]
    pub fn id(&self) -> u32 {
        match self {
            ColliderShapeDesc::Ball(_) => 0,
            ColliderShapeDesc::Cylinder(_) => 1,
            ColliderShapeDesc::RoundCylinder(_) => 2,
            ColliderShapeDesc::Cone(_) => 3,
            ColliderShapeDesc::Cuboid(_) => 4,
            ColliderShapeDesc::Capsule(_) => 5,
            ColliderShapeDesc::Segment(_) => 6,
            ColliderShapeDesc::Triangle(_) => 7,
            ColliderShapeDesc::Trimesh(_) => 8,
            ColliderShapeDesc::Heightfield(_) => 9,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ColliderShapeDesc::Ball(Default::default())),
            1 => Ok(ColliderShapeDesc::Cylinder(Default::default())),
            2 => Ok(ColliderShapeDesc::RoundCylinder(Default::default())),
            3 => Ok(ColliderShapeDesc::Cone(Default::default())),
            4 => Ok(ColliderShapeDesc::Cuboid(Default::default())),
            5 => Ok(ColliderShapeDesc::Capsule(Default::default())),
            6 => Ok(ColliderShapeDesc::Segment(Default::default())),
            7 => Ok(ColliderShapeDesc::Triangle(Default::default())),
            8 => Ok(ColliderShapeDesc::Trimesh(Default::default())),
            9 => Ok(ColliderShapeDesc::Heightfield(Default::default())),
            _ => Err(format!("Invalid collider shape desc id {}!", id)),
        }
    }

    #[doc(hidden)]
    pub fn from_collider_shape(shape: &dyn Shape) -> Self {
        if let Some(ball) = shape.as_ball() {
            ColliderShapeDesc::Ball(BallDesc {
                radius: ball.radius,
            })
        } else if let Some(cylinder) = shape.as_cylinder() {
            ColliderShapeDesc::Cylinder(CylinderDesc {
                half_height: cylinder.half_height,
                radius: cylinder.radius,
            })
        } else if let Some(round_cylinder) = shape.as_round_cylinder() {
            ColliderShapeDesc::RoundCylinder(RoundCylinderDesc {
                half_height: round_cylinder.base_shape.half_height,
                radius: round_cylinder.base_shape.radius,
                border_radius: round_cylinder.border_radius,
            })
        } else if let Some(cone) = shape.as_cone() {
            ColliderShapeDesc::Cone(ConeDesc {
                half_height: cone.half_height,
                radius: cone.radius,
            })
        } else if let Some(cuboid) = shape.as_cuboid() {
            ColliderShapeDesc::Cuboid(CuboidDesc {
                half_extents: cuboid.half_extents,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            ColliderShapeDesc::Capsule(CapsuleDesc {
                begin: capsule.segment.a.coords,
                end: capsule.segment.b.coords,
                radius: capsule.radius,
            })
        } else if let Some(segment) = shape.downcast_ref::<Segment>() {
            ColliderShapeDesc::Segment(SegmentDesc {
                begin: segment.a.coords,
                end: segment.b.coords,
            })
        } else if let Some(triangle) = shape.as_triangle() {
            ColliderShapeDesc::Triangle(TriangleDesc {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            })
        } else if shape.as_trimesh().is_some() {
            ColliderShapeDesc::Trimesh(TrimeshDesc)
        } else if shape.as_heightfield().is_some() {
            ColliderShapeDesc::Heightfield(HeightfieldDesc)
        } else {
            unreachable!()
        }
    }

    fn into_collider_shape(self) -> SharedShape {
        match self {
            ColliderShapeDesc::Ball(ball) => SharedShape::ball(ball.radius),
            ColliderShapeDesc::Cylinder(cylinder) => {
                SharedShape::cylinder(cylinder.half_height, cylinder.radius)
            }
            ColliderShapeDesc::RoundCylinder(rcylinder) => SharedShape::round_cylinder(
                rcylinder.half_height,
                rcylinder.radius,
                rcylinder.border_radius,
            ),
            ColliderShapeDesc::Cone(cone) => SharedShape::cone(cone.half_height, cone.radius),
            ColliderShapeDesc::Cuboid(cuboid) => SharedShape::cuboid(
                cuboid.half_extents.x,
                cuboid.half_extents.y,
                cuboid.half_extents.z,
            ),
            ColliderShapeDesc::Capsule(capsule) => SharedShape::capsule(
                Point3::from(capsule.begin),
                Point3::from(capsule.end),
                capsule.radius,
            ),
            ColliderShapeDesc::Segment(segment) => {
                SharedShape::segment(Point3::from(segment.begin), Point3::from(segment.end))
            }
            ColliderShapeDesc::Triangle(triangle) => SharedShape::triangle(
                Point3::from(triangle.a),
                Point3::from(triangle.b),
                Point3::from(triangle.c),
            ),
            ColliderShapeDesc::Trimesh(_) => {
                // Create fake trimesh. It will be filled with actual data on resolve stage later on.
                let a = Point3::new(0.0, 0.0, 1.0);
                let b = Point3::new(1.0, 0.0, 1.0);
                let c = Point3::new(1.0, 0.0, 0.0);
                SharedShape::trimesh(vec![a, b, c], vec![[0, 1, 2]])
            }
            ColliderShapeDesc::Heightfield(_) => SharedShape::heightfield(
                DMatrix::from_data(VecStorage::new(
                    Dynamic::new(2),
                    Dynamic::new(2),
                    vec![0.0, 1.0, 0.0, 0.0],
                )),
                Default::default(),
            ),
        }
    }
}

impl Visit for ColliderShapeDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id: u32 = if visitor.is_reading() { 0 } else { self.id() };
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        match self {
            ColliderShapeDesc::Ball(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cylinder(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::RoundCylinder(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cone(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Cuboid(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Capsule(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Segment(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Triangle(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Trimesh(v) => v.visit(name, visitor)?,
            ColliderShapeDesc::Heightfield(v) => v.visit(name, visitor)?,
        }

        visitor.leave_region()
    }
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct ColliderDesc<R> {
    pub shape: ColliderShapeDesc,
    pub parent: R,
    pub friction: f32,
    pub density: Option<f32>,
    pub restitution: f32,
    pub is_sensor: bool,
    pub translation: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub collision_groups: InteractionGroupsDesc,
    pub solver_groups: InteractionGroupsDesc,
}

#[doc(hidden)]
#[derive(Visit, Debug, Clone, Copy)]
pub struct InteractionGroupsDesc {
    pub memberships: u32,
    pub filter: u32,
}

impl Default for InteractionGroupsDesc {
    fn default() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }
}

impl From<InteractionGroups> for InteractionGroupsDesc {
    fn from(g: InteractionGroups) -> Self {
        Self {
            memberships: g.memberships,
            filter: g.filter,
        }
    }
}

impl<R: Default> Default for ColliderDesc<R> {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            parent: Default::default(),
            friction: 0.5,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            translation: Default::default(),
            rotation: Default::default(),
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }
}

impl<R: Hash + Clone + Eq> ColliderDesc<R> {
    pub(super) fn from_collider(
        collider: &Collider,
        handle_map: &BiDirHashMap<R, rapier3d::dynamics::RigidBodyHandle>,
    ) -> Self {
        Self {
            shape: ColliderShapeDesc::from_collider_shape(collider.shape()),
            parent: handle_map
                .key_of(&collider.parent().unwrap())
                .cloned()
                .unwrap(),
            friction: collider.friction(),
            density: collider.density(),
            restitution: collider.restitution(),
            is_sensor: collider.is_sensor(),
            translation: collider.position_wrt_parent().unwrap().translation.vector,
            rotation: collider.position_wrt_parent().unwrap().rotation,
            collision_groups: collider.collision_groups().into(),
            solver_groups: collider.solver_groups().into(),
        }
    }

    pub(super) fn convert_to_collider(self) -> (Collider, R) {
        let mut builder = ColliderBuilder::new(self.shape.into_collider_shape())
            .friction(self.friction)
            .restitution(self.restitution)
            .position(Isometry3 {
                translation: Translation3 {
                    vector: self.translation,
                },
                rotation: self.rotation,
            })
            .solver_groups(InteractionGroups::new(
                self.solver_groups.memberships,
                self.solver_groups.filter,
            ))
            .collision_groups(InteractionGroups::new(
                self.collision_groups.memberships,
                self.collision_groups.memberships,
            ))
            .sensor(self.is_sensor);
        if let Some(density) = self.density {
            builder = builder.density(density);
        }
        (builder.build(), self.parent)
    }
}

impl<R: 'static + Visit + Default> Visit for ColliderDesc<R> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.shape.visit("Shape", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.friction.visit("Friction", visitor)?;
        self.restitution.visit("Restitution", visitor)?;
        self.is_sensor.visit("IsSensor", visitor)?;
        self.translation.visit("Translation", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        let _ = self.collision_groups.visit("CollisionGroups", visitor);
        let _ = self.solver_groups.visit("SolverGroups", visitor);
        self.density.visit("Density", visitor)?;

        visitor.leave_region()
    }
}

// Almost full copy of rapier's IntegrationParameters
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct IntegrationParametersDesc {
    pub dt: f32,
    pub erp: f32,
    pub min_ccd_dt: f32,
    pub joint_erp: f32,
    pub warmstart_coeff: f32,
    pub warmstart_correction_slope: f32,
    pub velocity_solve_fraction: f32,
    pub velocity_based_erp: f32,
    pub allowed_linear_error: f32,
    pub prediction_distance: f32,
    pub allowed_angular_error: f32,
    pub max_linear_correction: f32,
    pub max_angular_correction: f32,
    pub max_velocity_iterations: u32,
    pub max_position_iterations: u32,
    pub min_island_size: u32,
    pub max_ccd_substeps: u32,
}

impl Default for IntegrationParametersDesc {
    fn default() -> Self {
        Self::from(IntegrationParameters::default())
    }
}

impl From<IntegrationParameters> for IntegrationParametersDesc {
    fn from(params: IntegrationParameters) -> Self {
        Self {
            dt: params.dt,
            erp: params.erp,
            min_ccd_dt: params.min_ccd_dt,
            joint_erp: params.joint_erp,
            warmstart_coeff: params.warmstart_coeff,
            warmstart_correction_slope: params.warmstart_correction_slope,
            velocity_solve_fraction: params.velocity_solve_fraction,
            velocity_based_erp: params.velocity_based_erp,
            allowed_linear_error: params.allowed_linear_error,
            prediction_distance: params.prediction_distance,
            allowed_angular_error: params.allowed_angular_error,
            max_linear_correction: params.max_linear_correction,
            max_angular_correction: params.max_angular_correction,
            max_velocity_iterations: params.max_velocity_iterations as u32,
            max_position_iterations: params.max_position_iterations as u32,
            min_island_size: params.min_island_size as u32,
            max_ccd_substeps: params.max_ccd_substeps as u32,
        }
    }
}

impl Into<IntegrationParameters> for IntegrationParametersDesc {
    fn into(self) -> IntegrationParameters {
        IntegrationParameters {
            dt: self.dt,
            min_ccd_dt: self.min_ccd_dt,
            erp: self.erp,
            joint_erp: self.joint_erp,
            warmstart_coeff: self.warmstart_coeff,
            warmstart_correction_slope: self.warmstart_correction_slope,
            velocity_solve_fraction: self.velocity_solve_fraction,
            velocity_based_erp: self.velocity_based_erp,
            allowed_linear_error: self.allowed_linear_error,
            allowed_angular_error: self.allowed_angular_error,
            max_linear_correction: self.max_linear_correction,
            max_angular_correction: self.max_angular_correction,
            prediction_distance: self.prediction_distance,
            max_velocity_iterations: self.max_velocity_iterations as usize,
            max_position_iterations: self.max_position_iterations as usize,
            min_island_size: self.min_island_size as usize,
            max_ccd_substeps: self.max_ccd_substeps as usize,
        }
    }
}

impl Visit for IntegrationParametersDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.dt.visit("DeltaTime", visitor)?;
        self.min_ccd_dt.visit("MinCcdDt", visitor)?;
        self.erp.visit("Erp", visitor)?;
        self.joint_erp.visit("JointErp", visitor)?;
        self.warmstart_coeff.visit("WarmstartCoeff", visitor)?;
        self.warmstart_correction_slope
            .visit("WarmstartCorrectionSlope", visitor)?;
        self.velocity_solve_fraction
            .visit("VelocitySolveFraction", visitor)?;
        self.velocity_based_erp.visit("VelocityBasedErp", visitor)?;
        self.allowed_linear_error
            .visit("AllowedLinearError", visitor)?;
        self.max_linear_correction
            .visit("MaxLinearCorrection", visitor)?;
        self.max_angular_correction
            .visit("MaxAngularCorrection", visitor)?;
        self.max_velocity_iterations
            .visit("MaxVelocityIterations", visitor)?;
        self.max_position_iterations
            .visit("MaxPositionIterations", visitor)?;
        self.min_island_size.visit("MinIslandSize", visitor)?;
        self.max_ccd_substeps.visit("MaxCcdSubsteps", visitor)?;

        // TODO: Remove
        if self.min_island_size == 0 {
            self.min_island_size = 128;
        }

        visitor.leave_region()
    }
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct BallJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct FixedJointDesc {
    pub local_anchor1_translation: Vector3<f32>,
    pub local_anchor1_rotation: UnitQuaternion<f32>,
    pub local_anchor2_translation: Vector3<f32>,
    pub local_anchor2_rotation: UnitQuaternion<f32>,
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct PrismaticJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_axis1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
    pub local_axis2: Vector3<f32>,
}

#[derive(Default, Clone, Debug, Visit)]
#[doc(hidden)]
pub struct RevoluteJointDesc {
    pub local_anchor1: Vector3<f32>,
    pub local_axis1: Vector3<f32>,
    pub local_anchor2: Vector3<f32>,
    pub local_axis2: Vector3<f32>,
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub enum JointParamsDesc {
    BallJoint(BallJointDesc),
    FixedJoint(FixedJointDesc),
    PrismaticJoint(PrismaticJointDesc),
    RevoluteJoint(RevoluteJointDesc),
}

impl Default for JointParamsDesc {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

impl Into<JointParams> for JointParamsDesc {
    fn into(self) -> JointParams {
        match self {
            JointParamsDesc::BallJoint(v) => JointParams::from(BallJoint::new(
                Point3::from(v.local_anchor1),
                Point3::from(v.local_anchor2),
            )),
            JointParamsDesc::FixedJoint(v) => JointParams::from(FixedJoint::new(
                Isometry3 {
                    translation: Translation3 {
                        vector: v.local_anchor1_translation,
                    },
                    rotation: v.local_anchor1_rotation,
                },
                Isometry3 {
                    translation: Translation3 {
                        vector: v.local_anchor2_translation,
                    },
                    rotation: v.local_anchor2_rotation,
                },
            )),
            JointParamsDesc::PrismaticJoint(v) => JointParams::from(PrismaticJoint::new(
                Point3::from(v.local_anchor1),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis1),
                Default::default(), // TODO
                Point3::from(v.local_anchor2),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis2),
                Default::default(), // TODO
            )),
            JointParamsDesc::RevoluteJoint(v) => JointParams::from(RevoluteJoint::new(
                Point3::from(v.local_anchor1),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis1),
                Point3::from(v.local_anchor2),
                Unit::<Vector3<f32>>::new_normalize(v.local_axis2),
            )),
        }
    }
}

impl JointParamsDesc {
    #[doc(hidden)]
    pub fn id(&self) -> u32 {
        match self {
            JointParamsDesc::BallJoint(_) => 0,
            JointParamsDesc::FixedJoint(_) => 1,
            JointParamsDesc::PrismaticJoint(_) => 2,
            JointParamsDesc::RevoluteJoint(_) => 3,
        }
    }

    #[doc(hidden)]
    pub fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::BallJoint(Default::default())),
            1 => Ok(Self::FixedJoint(Default::default())),
            2 => Ok(Self::PrismaticJoint(Default::default())),
            3 => Ok(Self::RevoluteJoint(Default::default())),
            _ => Err(format!("Invalid joint param desc id {}!", id)),
        }
    }
}

impl Visit for JointParamsDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }
        match self {
            JointParamsDesc::BallJoint(v) => v.visit("Data", visitor)?,
            JointParamsDesc::FixedJoint(v) => v.visit("Data", visitor)?,
            JointParamsDesc::PrismaticJoint(v) => v.visit("Data", visitor)?,
            JointParamsDesc::RevoluteJoint(v) => v.visit("Data", visitor)?,
        }

        visitor.leave_region()
    }
}

impl JointParamsDesc {
    #[doc(hidden)]
    pub fn from_params(params: &JointParams) -> Self {
        match params {
            JointParams::BallJoint(v) => Self::BallJoint(BallJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_anchor2: v.local_anchor2.coords,
            }),
            JointParams::FixedJoint(v) => Self::FixedJoint(FixedJointDesc {
                local_anchor1_translation: v.local_frame1.translation.vector,
                local_anchor1_rotation: v.local_frame1.rotation,
                local_anchor2_translation: v.local_frame2.translation.vector,
                local_anchor2_rotation: v.local_frame2.rotation,
            }),
            JointParams::PrismaticJoint(v) => Self::PrismaticJoint(PrismaticJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_axis1: v.local_axis1().into_inner(),
                local_anchor2: v.local_anchor2.coords,
                local_axis2: v.local_axis2().into_inner(),
            }),
            JointParams::RevoluteJoint(v) => Self::RevoluteJoint(RevoluteJointDesc {
                local_anchor1: v.local_anchor1.coords,
                local_axis1: v.local_axis1.into_inner(),
                local_anchor2: v.local_anchor2.coords,
                local_axis2: v.local_axis2.into_inner(),
            }),
        }
    }
}

#[derive(Clone, Debug, Default, Visit)]
#[doc(hidden)]
pub struct JointDesc<R> {
    pub body1: R,
    pub body2: R,
    pub params: JointParamsDesc,
}

impl<R: Hash + Clone + Eq> JointDesc<R> {
    #[doc(hidden)]
    pub fn from_joint(
        joint: &Joint,
        handle_map: &BiDirHashMap<R, rapier3d::dynamics::RigidBodyHandle>,
    ) -> Self {
        Self {
            body1: handle_map.key_of(&joint.body1).cloned().unwrap(),
            body2: handle_map.key_of(&joint.body2).cloned().unwrap(),
            params: JointParamsDesc::from_params(&joint.params),
        }
    }
}

#[derive(Default, Clone, Debug)]
#[doc(hidden)]
pub struct PhysicsDesc {
    pub integration_parameters: IntegrationParametersDesc,
    pub colliders: Vec<ColliderDesc<RigidBodyHandle>>,
    pub bodies: Vec<RigidBodyDesc<ColliderHandle>>,
    pub gravity: Vector3<f32>,
    pub joints: Vec<JointDesc<RigidBodyHandle>>,
    pub body_handle_map: BiDirHashMap<RigidBodyHandle, rapier3d::dynamics::RigidBodyHandle>,
    pub collider_handle_map: BiDirHashMap<ColliderHandle, rapier3d::geometry::ColliderHandle>,
    pub joint_handle_map: BiDirHashMap<JointHandle, rapier3d::dynamics::JointHandle>,
}

impl Visit for PhysicsDesc {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.integration_parameters
            .visit("IntegrationParameters", visitor)?;
        self.gravity.visit("Gravity", visitor)?;
        self.colliders.visit("Colliders", visitor)?;
        self.bodies.visit("Bodies", visitor)?;
        self.joints.visit("Joints", visitor)?;

        // TODO: Refactor duplicates here.
        {
            let mut body_handle_map = if visitor.is_reading() {
                Default::default()
            } else {
                let mut hash_map: HashMap<RigidBodyHandle, ErasedHandle> = Default::default();
                for (k, v) in self.body_handle_map.forward_map().iter() {
                    let (index, gen) = v.into_raw_parts();
                    hash_map.insert(*k, ErasedHandle::new(index as u32, gen as u32));
                }
                hash_map
            };
            body_handle_map.visit("BodyHandleMap", visitor)?;
            if visitor.is_reading() {
                self.body_handle_map = BiDirHashMap::from(
                    body_handle_map
                        .iter()
                        .map(|(k, v)| {
                            (
                                *k,
                                rapier3d::dynamics::RigidBodyHandle::from_raw_parts(
                                    v.index(),
                                    v.generation(),
                                ),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                );
            }
        }

        {
            let mut collider_handle_map = if visitor.is_reading() {
                Default::default()
            } else {
                let mut hash_map: HashMap<ColliderHandle, ErasedHandle> = Default::default();
                for (k, v) in self.collider_handle_map.forward_map().iter() {
                    let (index, gen) = v.into_raw_parts();
                    hash_map.insert(*k, ErasedHandle::new(index as u32, gen as u32));
                }
                hash_map
            };
            collider_handle_map.visit("ColliderHandleMap", visitor)?;
            if visitor.is_reading() {
                self.collider_handle_map = BiDirHashMap::from(
                    collider_handle_map
                        .iter()
                        .map(|(k, v)| {
                            (
                                *k,
                                rapier3d::geometry::ColliderHandle::from_raw_parts(
                                    v.index(),
                                    v.generation(),
                                ),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                );
            }
        }

        {
            let mut joint_handle_map = if visitor.is_reading() {
                Default::default()
            } else {
                let mut hash_map: HashMap<JointHandle, ErasedHandle> = Default::default();
                for (k, v) in self.joint_handle_map.forward_map().iter() {
                    let (index, gen) = v.into_raw_parts();
                    hash_map.insert(*k, ErasedHandle::new(index as u32, gen as u32));
                }
                hash_map
            };
            joint_handle_map.visit("JointHandleMap", visitor)?;
            if visitor.is_reading() {
                self.joint_handle_map = BiDirHashMap::from(
                    joint_handle_map
                        .iter()
                        .map(|(k, v)| {
                            (
                                *k,
                                rapier3d::dynamics::JointHandle::from_raw_parts(
                                    v.index(),
                                    v.generation(),
                                ),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                );
            }
        }

        visitor.leave_region()
    }
}
