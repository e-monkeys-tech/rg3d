//! Sphere emitter uniformly places particles in spherical volume. Can be used with
//! radius = 0, then it represents point emitter.   

use crate::{
    core::{algebra::Vector3, numeric_range::NumericRange, visitor::prelude::*},
    scene::particle_system::{
        emitter::{
            base::{BaseEmitter, BaseEmitterBuilder},
            Emit, Emitter,
        },
        particle::Particle,
        ParticleSystem,
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Clone, Visit)]
pub struct SphereEmitter {
    emitter: BaseEmitter,
    radius: f32,
}

impl Deref for SphereEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.emitter
    }
}

impl DerefMut for SphereEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.emitter
    }
}

impl Default for SphereEmitter {
    fn default() -> Self {
        Self {
            emitter: BaseEmitter::default(),
            radius: 0.5,
        }
    }
}

impl SphereEmitter {
    /// Creates new sphere emitter with given radius.
    pub fn new(emitter: BaseEmitter, radius: f32) -> Self {
        Self { emitter, radius }
    }

    /// Returns current radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets new sphere radius.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.max(0.0);
    }
}

impl Emit for SphereEmitter {
    fn emit(&self, _particle_system: &ParticleSystem, particle: &mut Particle) {
        self.emitter.emit(particle);
        let phi = NumericRange::new(0.0, std::f32::consts::PI).random();
        let theta = NumericRange::new(0.0, 2.0 * std::f32::consts::PI).random();
        let radius = NumericRange::new(0.0, self.radius).random();
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        particle.position = self.position()
            + Vector3::new(
                radius * sin_theta * cos_phi,
                radius * sin_theta * sin_phi,
                radius * cos_theta,
            );
    }
}

/// Sphere emitter builder allows you to construct sphere emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct SphereEmitterBuilder {
    base: BaseEmitterBuilder,
    radius: f32,
}

impl SphereEmitterBuilder {
    /// Creates new sphere emitter builder with 0.5 radius.
    pub fn new(base: BaseEmitterBuilder) -> Self {
        Self { base, radius: 0.5 }
    }

    /// Sets desired radius of sphere emitter.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Creates new sphere emitter.
    pub fn build(self) -> Emitter {
        Emitter::Sphere(SphereEmitter {
            emitter: self.base.build(),
            radius: self.radius,
        })
    }
}
