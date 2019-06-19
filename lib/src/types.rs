use nalgebra;
use std::mem;
use std::thread::{self, ThreadId};
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::marker::PhantomData;
use std::collections::HashMap;
use hibitset::BitSetLike;
use specs_derive::{Component};
use specs::prelude::*;
use specs::storage::{MaskedStorage, TryDefault, UnprotectedStorage};
use specs::world::Index as SpecsIndex;

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


// COLLECTOR TYPE FOR STORING ASSETS DATA

pub trait Id {
    fn new(id: usize) -> Self;

    fn get(&self) -> usize;
}

/// Meant to be used as resource
pub struct Collector<T, I> where I: Id{
    items: Vec<T>,
    name_to_id: HashMap<String, usize>,
    index_type: PhantomData<I>
}

impl<T, I> Collector<T, I> where I: Id {
    pub fn new_empty() -> Self {
        Collector {
            items: vec![],
            name_to_id: HashMap::new(),
            index_type: PhantomData
        }
    }

    /// save image by it's name and return acces index
    pub fn add_item(&mut self, name: String, data: T) -> I {
        self.items.push(data);
        let id = self.items.len() - 1;
        self.name_to_id.insert(name, id);
        Id::new(id)
    }

    pub fn get_item(&self, id: I) -> Option<&T> {
        if id.get() < self.items.len() {
            Some(&self.items[id.get()])
        } else {
            None
        }
    }
}

impl<T, I> Index<I> for Collector<T, I>  where I: Id{
    type Output = T;
    fn index<'a>(&'a self, id: I) -> &'a T {
        &self.items[id.get()]
    }
}

impl<T, I> IndexMut<I> for Collector<T, I>  where I: Id{
    fn index_mut<'a>(&'a mut self, id: I) -> &'a mut  T {
        &mut self.items[id.get()]
    }
}

// Retained storage

pub trait Retained<C> {
    fn retained(&mut self) -> Vec<C>;
}

impl<'e, T, D> Retained<T> for Storage<'e, T, D>
where
    T: Component,
    T::Storage: Retained<T>,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    fn retained(&mut self) -> Vec<T> {
        unsafe { self.open().1.retained() }
    }
}

pub struct RetainedStorage<C, T = UnprotectedStorage<C>> {
    retained: Vec<C>,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T> Default for RetainedStorage<C, T>
where
    T: TryDefault,
{
    fn default() -> Self {
        RetainedStorage {
            retained: vec![],
            storage: T::try_default().unwrap(),
            phantom: PhantomData,
        }
    }
}

impl<C, T> Retained<C> for RetainedStorage<C, T> {
    fn retained(&mut self) -> Vec<C> {
        mem::replace(&mut self.retained, vec![])
    }
}

impl<C: Clone, T: UnprotectedStorage<C>> UnprotectedStorage<C> for RetainedStorage<C, T> {
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        self.storage.clean(has)
    }

    unsafe fn get(&self, id: SpecsIndex) -> &C {
        self.storage.get(id)
    }

    unsafe fn get_mut(&mut self, id: SpecsIndex) -> &mut C {
        self.storage.get_mut(id)
    }

    unsafe fn insert(&mut self, id: SpecsIndex, comp: C) {
        self.storage.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: SpecsIndex) -> C {
        let comp = self.storage.remove(id);
        self.retained.push(comp.clone());
        comp
    }
}