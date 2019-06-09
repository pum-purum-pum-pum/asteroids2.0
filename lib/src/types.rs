use nalgebra;
use std::thread::{self, ThreadId};
use std::ops::{Deref, DerefMut};
use specs_derive::{Component};
use specs::prelude::*;

pub use nalgebra::Rotation3;
pub use nalgebra::Rotation2;
pub use nalgebra::Unit;
pub type Perspective3 =  nalgebra::Perspective3<f32>;
pub type Point2 = nalgebra::Point2<f32>;
pub type Point3 = nalgebra::Point3<f32>;
pub type Point4 = nalgebra::Point4<f32>;
pub type Matrix4 = nalgebra::Matrix4<f32>;
pub type Vector2 = nalgebra::Vector2<f32>;
pub type Vector3 = nalgebra::Vector3<f32>;
pub type Isometry3 = nalgebra::Isometry3<f32>;
pub type Isometry2 = nalgebra::Isometry2<f32>;

pub use ncollide2d::query::Ray;
pub type Segment = ncollide2d::shape::Segment<f32>;
pub use ncollide2d::query::ray_internal::ray::RayCast;

unsafe impl<T> Send for ThreadPin<T> {}
unsafe impl<T> Sync for ThreadPin<T> {}

/// Allows safaly implement Sync and Send for type T
/// panics if called from another thread
#[derive(Component)]
pub struct ThreadPin<T> where T: 'static {
    owner: ThreadId,
    inner: T
}

impl<T> ThreadPin<T> {
    pub fn new(value: T) -> Self {
        ThreadPin {
            owner: thread::current().id(),
            inner: value,
        }
    }
}

impl<T> Deref for ThreadPin<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        assert!(thread::current().id() == self.owner);
        &self.inner
    }
}

impl<T> DerefMut for ThreadPin<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(thread::current().id() == self.owner);
        &mut self.inner
    }
}

/// Option ThreadPin with deref(panics if None)
/// Allows to implement Default on ThreadPin
#[derive(Default)]
pub struct ThreadPinResource<T> where T: 'static {
    inner: Option<ThreadPin<T>>
}

impl<T> Deref for ThreadPinResource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().map(|x| x.deref()).unwrap()
    }
}

impl<T> DerefMut for ThreadPinResource<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().map(|x| x.deref_mut()).unwrap()
    }
}