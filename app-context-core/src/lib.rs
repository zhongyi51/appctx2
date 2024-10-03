pub mod error;
pub mod util;

use std::{
    any::{type_name, Any},
    collections::HashMap,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{Arc, LazyLock, RwLock, Weak},
};

use crate::error::{AppContextError, AppContextResult};
use util::weak_to_ref;

/// types define
pub type DynBuilder = Arc<
    dyn Fn(&AppContext) -> Pin<Box<dyn Future<Output = Arc<dyn AbstractAppObject>>>>
        + Send
        + Sync
        + 'static,
>;

/// structs define
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct BaseInfo {
    name: &'static str,
    type_name: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectMeta {
    type_info: BaseInfo,
    deps: &'static [BaseInfo],
    can_cast_to: &'static [BaseInfo],
}

impl ObjectMeta {
    fn compat_with(&self, expected: BaseInfo) -> bool {
        if self.type_info == expected {
            return true;
        }
        if self.can_cast_to.contains(&expected) {
            return true;
        }
        return false;
    }

    fn depends_on(&self, expected: BaseInfo) -> bool {
        self.deps.contains(&expected)
    }
}

pub trait AbstractAppObject: Any + Send + Sync + 'static {
    fn try_cast_to(&self, type_name: &'static str) -> AppContextResult<Box<dyn Any + '_>>;

    fn get_meta(&self) -> &'static ObjectMeta;
}

/// constructor for derive macros
pub struct ObjDynConstructor {
    meta: ObjectMeta,
    builder: DynBuilder,
}

inventory::collect!(ObjDynConstructor);

pub struct AppContextBuilder {
    inner: AppContext,
}

impl AppContextBuilder {
    pub fn new() -> Self {
        AppContextBuilder {
            inner: AppContext {
                inner: Default::default(),
            },
        }
    }

    pub async fn collected() -> Self {
        let mut builder = Self::new();
        for constructor in inventory::iter::<ObjDynConstructor>() {
            builder.register_dyn(constructor.builder.clone()).await;
        }
        builder
    }

    pub async fn register<Fut, T>(&mut self, builder: impl FnOnce(&AppContext) -> Fut)
    where
        Fut: Future<Output = T>,
        T: AbstractAppObject,
    {
        let val = builder(&self.inner).await;
        let mut_inner =
            Arc::get_mut(&mut self.inner.inner).expect("builder inner is wrongly cloned");
        mut_inner.register_dyn(Arc::new(val));
    }

    pub async fn register_dyn(&mut self, dyn_builder: DynBuilder) {
        let built = dyn_builder(&self.inner).await;
        let mut_inner =
            Arc::get_mut(&mut self.inner.inner).expect("builder inner is wrongly cloned");
        mut_inner.register_dyn(built);
    }
}

#[derive(Clone)]
pub struct AppContext {
    inner: Arc<AppContextInner>,
}

impl AppContext {
    pub fn get_obj<T: AbstractAppObject>(&self, expected: BaseInfo) -> Option<AppObjectRef<T>> {
        self.inner
            .get_and_cache_inner(expected)
            .map(|weak| AppObjectRef::new(weak, expected))
    }

    pub fn get_lazy_obj<T: AbstractAppObject>(&self, expected: BaseInfo) -> LazyAppObjectRef<T> {
        let inner = Arc::downgrade(&self.inner);
        LazyAppObjectRef::lazy_new(inner, expected).unwrap()
    }
}

#[derive(Default)]
pub struct AppContextInner {
    objects: Vec<Arc<dyn AbstractAppObject>>,
    object_cache: RwLock<HashMap<&'static str, HashMap<&'static str, Weak<dyn AbstractAppObject>>>>,
}

impl AppContextInner {
    fn register_dyn(&mut self, obj: Arc<dyn AbstractAppObject>) {
        self.objects.push(obj);
    }

    fn get_and_cache_inner(&self, expected: BaseInfo) -> Option<Weak<dyn AbstractAppObject>> {
        match self.object_cache.read() {
            Ok(r) => {
                if let Some(cached_map) = r.get(expected.type_name) {
                    if let Some(cached_obj) = cached_map.get(expected.name) {
                        return Some(cached_obj.clone());
                    }
                }
            }
            Err(_) => {
                panic!("unexpected lock error");
            }
        }
        let target = self
            .objects
            .iter()
            .find(|s| s.get_meta().compat_with(expected))
            .map(|p| Arc::downgrade(p));

        if let Some(weak_ptr) = &target {
            match self.object_cache.write() {
                Ok(mut w) => {
                    let _ = w
                        .entry(expected.type_name)
                        .or_insert(HashMap::new())
                        .entry(expected.name)
                        .or_insert(weak_ptr.clone());
                }
                Err(_) => {
                    panic!("unexpected lock error");
                }
            }
        }
        return target;
    }
}

pub struct AppObjectRef<T: ?Sized> {
    inner: Weak<dyn AbstractAppObject>,
    base_info: BaseInfo,
    _marker: std::marker::PhantomData<T>,
}

impl<T> AppObjectRef<T>
where
    T: ?Sized + 'static,
{
    pub fn new(arc: Weak<dyn AbstractAppObject>, base_info: BaseInfo) -> Self {
        AppObjectRef {
            inner: arc,
            base_info,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn try_downcast(&self) -> AppContextResult<&T> {
        cast_ref(&self.inner, self.base_info)
    }
}

impl<T> Deref for AppObjectRef<T>
where
    T: ?Sized + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.try_downcast().expect("downcast err")
    }
}

pub struct LazyAppObjectRef<T: ?Sized> {
    inner: LazyLock<
        Option<Weak<dyn AbstractAppObject>>,
        Box<dyn FnOnce() -> Option<Weak<dyn AbstractAppObject>>>,
    >,
    base_info: BaseInfo,
    marker: std::marker::PhantomData<T>,
}

impl<T> LazyAppObjectRef<T>
where
    T: ?Sized + 'static,
{
    pub fn lazy_new(arc: Weak<AppContextInner>, base_info: BaseInfo) -> AppContextResult<Self> {
        let init_f = move || {
            let inner_ref = weak_to_ref(&arc)?;
            let obj: Weak<dyn AbstractAppObject> = inner_ref.get_and_cache_inner(base_info)?;
            Some(obj)
        };
        Ok(LazyAppObjectRef {
            inner: LazyLock::new(Box::new(init_f)),
            base_info,
            marker: std::marker::PhantomData,
        })
    }

    pub fn try_downcast(&self) -> AppContextResult<&T> {
        let weak_ptr = self.inner.as_ref().ok_or(AppContextError::ObjectNotFound {
            obj_name: self.base_info.name,
            obj_type: self.base_info.type_name,
        })?;
        cast_ref(weak_ptr, self.base_info)
    }
}

fn cast_ref<T: ?Sized + 'static>(
    weak_ptr: &Weak<dyn AbstractAppObject>,
    base_info: BaseInfo,
) -> AppContextResult<&T> {
    let Some(r_ref) = weak_to_ref(weak_ptr) else {
        return Err(AppContextError::AppContextDropped);
    };
    let cast_any = r_ref.try_cast_to(type_name::<T>())?;
    let actual_ref = cast_any
        .downcast::<&T>()
        .map_err(|_| AppContextError::UnsupportedCast {
            obj_name: base_info.name,
            obj_type: base_info.type_name,
            expected_type: type_name::<T>(),
        })?;
    Ok(*actual_ref)
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
